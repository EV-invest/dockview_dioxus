//! A live, multi-group docking layout — the visual proof that the engine actually docks.
//! Run with `dx serve --example basic --package dockview_dioxus`.
//!
//! What it demonstrates, all driven by the same model the drag code mutates:
//! - a seeded 4-group split (both axes) via `on_ready` calling the public `move_panel`,
//! - drag a tab onto another group's edge/centre to re-dock or split,
//! - drag the splitters to resize, double-click a titlebar to maximize,
//! - the "float" button detaches a group; drag its titlebar back over the grid to re-dock,
//! - type into **Notes**, then drag it around — the text survives, because panel instances
//!   live in the stable content overlay and are never remounted on restructuring.

use dioxus::prelude::*;
use dockview_dioxus::{DockApi, DockArea, DockPanel, GroupId, PanelId, Position, math::Rect};

fn main() {
	dioxus::launch(app);
}

fn app() -> Element {
	let panels = vec![
		DockPanel {
			id: PanelId("watchlist".into()),
			title: "Watchlist".into(),
			content: rsx! { Watchlist {} },
		},
		DockPanel {
			id: PanelId("notes".into()),
			title: "Notes".into(),
			content: rsx! { Notes {} },
		},
		DockPanel {
			id: PanelId("chart".into()),
			title: "Chart".into(),
			content: rsx! { Chart {} },
		},
		DockPanel {
			id: PanelId("orders".into()),
			title: "Orders".into(),
			content: rsx! { Orders {} },
		},
		DockPanel {
			id: PanelId("console".into()),
			title: "Console".into(),
			content: rsx! { Console {} },
		},
	];

	// Split the default single group into:  ┌ Watchlist+Notes ┬ Chart ┐
	//                                        └ Orders         ┴ Console┘
	// Each call goes through the same `apply_drop` path a real drag uses.
	let seed = Callback::new(move |api: DockApi| {
		let mut api = api;
		api.move_panel(PanelId("chart".into()), vec![], Position::Right);
		api.move_panel(PanelId("console".into()), vec![1], Position::Bottom);
		api.move_panel(PanelId("orders".into()), vec![0], Position::Bottom);
	});

	rsx! {
		// The dock fills its parent (height:100%); pin a full-viewport host so it has a real
		// size to measure against (otherwise height:100% collapses to 0 and nothing shows).
		div { style: "position:fixed; inset:0; background:#1e1e1e;",
			DockArea {
				panels,
				storage_key: Some("dockview-finance-demo".to_string()),
				on_ready: Some(seed),
			}
		}
	}
}

#[component]
fn Watchlist() -> Element {
	let rows = [
		("BTC/USD", "67,536.1", "-0.03%"),
		("AAPL", "182.41", "+0.07%"),
		("MSFT", "413.96", "-0.09%"),
		("NVDA", "876.13", "+0.06%"),
		("TSLA", "248.95", "+0.08%"),
	];
	rsx! {
		div { style: "padding:8px; font:12px monospace;",
			for (sym , px , chg) in rows {
				div { style: "display:flex; justify-content:space-between; padding:2px 0;",
					span { "{sym}" }
					span { "{px}" }
					span { style: if chg.starts_with('+') { "color:#4caf50" } else { "color:#f44" }, "{chg}" }
				}
			}
		}
	}
}

/// A textarea whose contents prove instance preservation: type, then drag this panel into
/// another group — the text stays, because the overlay never remounts the instance.
#[component]
fn Notes() -> Element {
	rsx! {
		textarea {
			style: "width:100%; height:100%; box-sizing:border-box; border:0; resize:none;
				background:#1e1e1e; color:#ddd; padding:8px; font:13px monospace;",
			placeholder: "Type here, then drag this tab to another group — the text survives.",
		}
	}
}

#[component]
fn Chart() -> Element {
	rsx! {
		div { style: "padding:8px; height:100%; box-sizing:border-box;",
			div { style: "font:600 14px monospace; color:#f44;", "BTC/USD  67,536.1" }
			div { style: "margin-top:8px; height:80%;
				background:linear-gradient(180deg,#2a2a2a,#1e1e1e);
				border:1px solid #333; display:flex; align-items:center; justify-content:center;
				color:#555; font:11px monospace;",
				"[ chart ]"
			}
		}
	}
}

#[component]
fn Orders() -> Element {
	let rows = [
		("NVDA", "Buy", "109", "Pending"),
		("CRM", "Sell", "1769", "Cancelled"),
		("NVDA", "Sell", "1541", "Filled"),
		("TSLA", "Sell", "499", "Cancelled"),
	];
	rsx! {
		div { style: "padding:8px; font:11px monospace;",
			for (sym , side , qty , status) in rows {
				div { style: "display:flex; gap:12px; padding:2px 0;",
					span { style: "width:48px;", "{sym}" }
					span { style: if side == "Buy" { "color:#4caf50; width:40px" } else { "color:#f44; width:40px" }, "{side}" }
					span { style: "width:48px; text-align:right;", "{qty}" }
					span { "{status}" }
				}
			}
		}
	}
}

/// Holds the only UI gesture that *creates* a float: detach the Watchlist+Notes group.
/// Drag the resulting floating titlebar back over the grid to re-dock it.
#[component]
fn Console() -> Element {
	let mut api = use_context::<DockApi>();
	rsx! {
		div { style: "padding:8px; font:12px monospace; color:#888;",
			div { "Console ready." }
			button {
				style: "margin-top:8px; cursor:pointer;",
				// GroupId(0) is the default group (Watchlist+Notes after the seed splits).
				onclick: move |_| api.float(GroupId(0), Rect { x: 120.0, y: 120.0, width: 320.0, height: 220.0 }),
				"float Watchlist group"
			}
		}
	}
}
