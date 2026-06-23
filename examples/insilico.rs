//! The packed-grid paradigm in action — InsilicoTerminal's tiled trading layout, the
//! visual proof of [`PackedArea`]. Run with
//! `dx serve --example insilico --package dockview_dioxus --platform web`.
//!
//! What it demonstrates (all over the same `PackedGrid` the chrome mutates):
//! - tiles have a *starting* size and pack left-to-right, leaving whitespace below,
//! - the top-bar `+` spawns a random-sized window that auto-packs into the slot growing
//!   the layout's lowest point the least,
//! - each tile's `+` opens a catalog window as a new **tab** in that tile,
//! - each tile's `x` closes the active tab (emptying it removes the tile),
//! - the bottom-right grip resizes in `STEP` increments, clamped against neighbours,
//! - mock prices/quantities tick live while you interact.

use dioxus::prelude::*;
use dockview_dioxus::{Config, DockPanel, Group, GroupId, Keybind, MinSize, PackedApi, PackedArea, PanelId, Step, persist};

/// localStorage key the layout round-trips through (see `dockview_dioxus::persist`/`serial`).
const STORAGE_KEY: &str = "insilico-layout";

fn main() {
	dioxus::launch(app);
}

/// xorshift64 — a tiny no-dep PRNG so random tile sizes/kinds need no `rand`.
fn xorshift(s: &mut u64) -> u64 {
	*s ^= *s << 13;
	*s ^= *s >> 7;
	*s ^= *s << 17;
	*s
}

/// The catalog of insilico-style windows. Each declares its title, default size, and a
/// per-type [`MinSize`] (in whichever unit reads naturally).
#[derive(Clone, Copy, PartialEq)]
enum Kind {
	Positions,
	Orders,
	OrderBook,
	Watchlist,
	Trades,
	Balances,
	Chart,
}

impl Kind {
	const ALL: [Kind; 7] = [Kind::Positions, Kind::Orders, Kind::OrderBook, Kind::Watchlist, Kind::Trades, Kind::Balances, Kind::Chart];

	fn id(self) -> &'static str {
		match self {
			Kind::Positions => "positions",
			Kind::Orders => "orders",
			Kind::OrderBook => "orderbook",
			Kind::Watchlist => "watchlist",
			Kind::Trades => "trades",
			Kind::Balances => "balances",
			Kind::Chart => "chart",
		}
	}

	fn title(self) -> &'static str {
		match self {
			Kind::Positions => "Positions",
			Kind::Orders => "Orders",
			Kind::OrderBook => "Order Book",
			Kind::Watchlist => "Watchlist",
			Kind::Trades => "Trades",
			Kind::Balances => "Balances",
			Kind::Chart => "Chart",
		}
	}

	fn min(self) -> MinSize {
		match self {
			Kind::Positions => MinSize::Rem { w: 10.0, h: 6.0 },
			Kind::Orders => MinSize::Steps { w: Step(3), h: Step(2) },
			Kind::OrderBook => MinSize::Rem { w: 8.0, h: 8.0 },
			Kind::Watchlist => MinSize::Steps { w: Step(2), h: Step(2) },
			Kind::Trades => MinSize::Rem { w: 9.0, h: 5.0 },
			Kind::Balances => MinSize::Steps { w: Step(2), h: Step(2) },
			Kind::Chart => MinSize::Rem { w: 12.0, h: 8.0 },
		}
	}
}

/// The catalog kind a panel id was minted from (ids are `"{kind.id}-{n}"`), used to rebuild
/// a panel's content after a layout is restored from storage.
fn kind_from_id(id: &str) -> Option<Kind> {
	let prefix = id.rsplit_once('-')?.0;
	Kind::ALL.into_iter().find(|k| k.id() == prefix)
}

/// A panel's content for `kind` under a given id.
fn panel_of(kind: Kind, id: PanelId) -> DockPanel {
	DockPanel {
		id,
		title: kind.title().into(),
		content: rsx! { KindView { kind } },
	}
}

/// A unique panel id + its content for `kind`, minted from the running counter.
fn make_panel(kind: Kind, counter: &mut Signal<u64>) -> DockPanel {
	let n = counter();
	counter.set(n + 1);
	panel_of(kind, PanelId(format!("{}-{n}", kind.id())))
}

/// Mint a window of `kind` and place it as a fresh tile.
fn spawn(kind: Kind, w: u32, h: u32, mut panels: Signal<Vec<DockPanel>>, mut counter: Signal<u64>, mut api: PackedApi) {
	let panel = make_panel(kind, &mut counter);
	let id = panel.id.clone();
	panels.write().push(panel);
	let gid = api.grid.write().mint_group_id();
	api.place(Group::new(gid, id), w, h, kind.min());
}

/// Host-registered chord: `Alt+S` notifies the user then stubs the "POST layout to server" pipe
/// by reading the live layout JSON the closure's [`PackedApi`] hands it. Proof the hook runs
/// arbitrary wasm and reaches the current layout — swap the `alert` for a real `fetch` to ship it.
fn insilico_config() -> Config {
	let save = Callback::new(|api: PackedApi| {
		let msg = format!("Alt+S: would POST {} bytes of layout", api.save().len());
		#[cfg(target_arch = "wasm32")]
		web_sys::window().expect("a browser window").alert_with_message(&msg).expect("alert");
		#[cfg(not(target_arch = "wasm32"))]
		println!("{msg}");
	});
	Config {
		actions: vec![(Keybind { key: "s", alt: true, ctrl: false }, save)],
		..Default::default()
	}
}

fn app() -> Element {
	let panels = use_signal(Vec::<DockPanel>::new);
	let counter = use_signal(|| 0u64);
	let mut tick = use_context_provider(|| Signal::new(0u64));
	// Set on ready, then drives every later interaction from outside `PackedArea`'s subtree.
	let mut api = use_signal(|| None::<PackedApi>);
	// A tile's `+` records its group here; the popup below reads it to anchor the catalog.
	let mut pending = use_signal(|| None::<GroupId>);

	// Per-tile `+` → open the catalog popup for that group.
	use_context_provider(|| Callback::new(move |gid: GroupId| pending.set(Some(gid))));

	// One scope-tied ticker bumps the shared clock; panels derive their mock values from it.
	use_future(move || async move {
		loop {
			gloo_timers::future::TimeoutFuture::new(600).await;
			let n = tick();
			tick.set(n + 1);
		}
	});

	// Persist the layout on every settled change; on reload it's restored below.
	use_effect(move || {
		if let Some(a) = api() {
			persist::write(STORAGE_KEY, &a.save());
		}
	});

	// Restore a saved layout (rebuilding panel content from the kind-encoded ids) if present,
	// else seed ~4 tiles mirroring the screenshot, leaving whitespace below.
	let seed = Callback::new(move |a: PackedApi| {
		let mut a = a;
		let mut panels = panels;
		let mut counter = counter;
		api.set(Some(a));
		if let Some(json) = persist::read(STORAGE_KEY) {
			if a.load(&json).is_ok() {
				let mut rebuilt = Vec::new();
				let mut next = 0u64;
				for cell in &a.grid.read().cells {
					for pid in &cell.group.tabs {
						if let Some(kind) = kind_from_id(&pid.0) {
							if let Some(n) = pid.0.rsplit_once('-').and_then(|(_, s)| s.parse::<u64>().ok()) {
								next = next.max(n + 1);
							}
							rebuilt.push(panel_of(kind, pid.clone()));
						}
					}
				}
				panels.set(rebuilt);
				counter.set(next);
				return;
			}
		}
		spawn(Kind::Watchlist, 12, 16, panels, counter, a);
		spawn(Kind::Positions, 16, 12, panels, counter, a);
		spawn(Kind::OrderBook, 12, 20, panels, counter, a);
		spawn(Kind::Chart, 20, 16, panels, counter, a);
	});

	let add_random = move |_| {
		let Some(a) = api() else { return };
		let mut s = counter().wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(0xd1b5);
		let kind = Kind::ALL[xorshift(&mut s) as usize % Kind::ALL.len()];
		let w = 8 + (xorshift(&mut s) % 16) as u32; // 8..=23
		let h = 8 + (xorshift(&mut s) % 12) as u32; // 8..=19
		spawn(kind, w, h, panels, counter, a);
	};

	rsx! {
		div { style: "position:fixed; inset:0; display:flex; flex-direction:column; background:#0b0f0e;",
			div {
				style: "flex:0 0 auto; height:44px; display:flex; align-items:center; gap:14px;
					padding:0 14px; background:#0f1b18; border-bottom:2px solid #63e9cd;",
				span { style: "color:#63e9cd; font:700 15px system-ui;", "InSilico" }
				button {
					style: "cursor:pointer; border:1px solid #63e9cd; background:transparent; color:#63e9cd;
						width:26px; height:26px; border-radius:4px; font:600 16px system-ui; line-height:1;",
					title: "Add a random window",
					onclick: add_random,
					"+"
				}
			}
			div { style: "flex:1 1 auto; position:relative;",
				PackedArea { panels, on_ready: Some(seed), config: Some(insilico_config()) }
			}
			if let Some(gid) = pending() {
				CatalogPopup { gid, panels, counter, api, pending }
			}
		}
	}
}

/// The per-tile `+` catalog: pick a kind to open it as a new tab in `gid`'s tile.
// ponytail: a centred modal, not anchored to the tile's rect — anchor it by threading the
// tile's grid rect through the request-tab callback if it ever matters.
#[component]
fn CatalogPopup(gid: GroupId, panels: Signal<Vec<DockPanel>>, counter: Signal<u64>, api: Signal<Option<PackedApi>>, pending: Signal<Option<GroupId>>) -> Element {
	let mut pending = pending;
	rsx! {
		div {
			style: "position:fixed; inset:0; z-index:2000; display:flex; align-items:center; justify-content:center;
				background:rgba(0,0,0,.45);",
			onclick: move |_| pending.set(None),
			div {
				style: "background:#0f1b18; border:1px solid #63e9cd; border-radius:6px; padding:10px; min-width:180px;",
				onclick: |e: MouseEvent| e.stop_propagation(),
				div { style: "color:#63e9cd; font:600 13px system-ui; margin-bottom:6px;", "Add window" }
				for kind in Kind::ALL {
					button {
						style: "display:block; width:100%; text-align:left; cursor:pointer; border:0; padding:6px 8px;
							background:transparent; color:#ddd; font:13px system-ui; border-radius:4px;",
						onclick: move |_| {
							let mut counter = counter;
							let mut panels = panels;
							let panel = make_panel(kind, &mut counter);
							let id = panel.id.clone();
							panels.write().push(panel);
							if let Some(mut a) = api() {
								a.add_tab(gid, id);
							}
							pending.set(None);
						},
						"{kind.title()}"
					}
				}
			}
		}
	}
}

/// A catalog window's body — color-coded rows of mock data that update with the shared tick.
#[component]
fn KindView(kind: Kind) -> Element {
	let tick = use_context::<Signal<u64>>();
	let t = tick();
	let syms = ["BTC", "ETH", "SOL", "AAPL", "NVDA", "TSLA", "MSFT", "AMD"];
	// A stable per-(kind,row) pseudo value that walks with the tick, so prices "move".
	let val = move |row: u64| -> i64 {
		let mut s = t.wrapping_mul(0x2545_f491_4f6c_dd1d).wrapping_add(row.wrapping_mul(0x9e37_79b9)).wrapping_add(kind as u64 + 1);
		(xorshift(&mut s) % 100_000) as i64
	};

	if kind == Kind::Chart {
		let sym = syms[(t % 8) as usize];
		let price = format!("{:.2}", val(0) as f64 / 100.0);
		let bars: Vec<u64> = (0..16u64).map(|i| 20 + (val(i) % 80) as u64).collect();
		return rsx! {
			div { style: "padding:6px 8px; height:100%; box-sizing:border-box;",
				div { style: "font:600 13px ui-monospace, monospace; color:#63e9cd;", "{sym}/USD  {price}" }
				div {
					style: "margin-top:6px; height:78%; border:1px solid #1d2c28;
						background:linear-gradient(180deg,#0f1b18,#0b0f0e); display:flex; align-items:flex-end; gap:2px; padding:4px;",
					for h in bars {
						div { style: "flex:1; background:#2c6; opacity:.7; height:{h}%;" }
					}
				}
			}
		};
	}

	let rows = if kind == Kind::Balances { 3u64 } else { 6 };
	let cells: Vec<(String, String, String, &'static str)> = (0..rows)
		.map(|r| {
			let v = val(r);
			let up = v % 2 == 0;
			let sym = syms[(r + kind as u64) as usize % syms.len()].to_string();
			let price = format!("{:.2}", v as f64 / 100.0);
			let delta = format!("{}{}", if up { "+" } else { "-" }, v % 1000);
			(sym, price, delta, if up { "#3ddc84" } else { "#ff5d6c" })
		})
		.collect();
	rsx! {
		div { style: "padding:6px 8px; font:12px ui-monospace, monospace; height:100%; box-sizing:border-box;",
			for (sym , price , delta , color) in cells {
				div { style: "display:flex; justify-content:space-between; gap:10px; padding:2px 0;",
					span { style: "color:#9fb;", "{sym}" }
					span { style: "color:{color};", "{price}" }
					span { style: "color:{color}; width:56px; text-align:right;", "{delta}" }
				}
			}
		}
	}
}
