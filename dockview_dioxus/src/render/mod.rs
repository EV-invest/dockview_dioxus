//! The Dioxus render layer — everything DOM. This is where dockview's per-class
//! `this.element` ownership is replaced by declarative `rsx!` derived from the model
//! `Signal`. Dockview infra that Dioxus subsumes is intentionally absent: no
//! `events.ts` Emitter (Signals), no `lifecycle.ts` Disposable (scopes/`use_drop`),
//! no `dom.ts` (rsx).
//!
//! Three stacked layers, painted back-to-front, mirroring dockview:
//! 1. [`grid`]     — recursive *skeleton*: nested flex divs, group frames, splitter
//!    handles. Holds **no** user content, so restructuring it is harmless.
//! 2. [`content`]  — flat, id-keyed *content overlay* (`OverlayRenderContainer`
//!    equivalent): one absolutely-positioned div per panel, positioned from the
//!    measured box of its group's content slot. Stable keys ⇒ instances never remount.
//! 3. [`floating`] / [`drop_overlay`] — floating groups and the live drop indicator.

pub mod content;
pub mod drop_overlay;
pub mod floating;
pub mod grid;
pub mod group;

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::{
	api::DockApi,
	math::Rect,
	model::{DockModel, GroupId, PanelMeta, dnd::DragState, gridview::GridNode, group::Group},
	panel::DockPanel,
};

/// Minimal structural stylesheet. Layout (flex/sizing) ships with the lib; all
/// colors/sizes read from `--dv-*` custom properties so a host can re-theme without
/// us hardcoding a palette. Not a port of dockview's full SCSS theme.
const CSS: &str = r#"
.dv-dockview { position: relative; width: 100%; height: 100%; overflow: hidden;
	color: var(--dv-fg, #ddd); font: 13px/1.4 system-ui, sans-serif; }
.dv-branch { display: flex; width: 100%; height: 100%; }
.dv-row { flex-direction: row; }
.dv-col { flex-direction: column; }
.dv-child { position: relative; overflow: hidden; flex-grow: 0; flex-shrink: 0;
	min-width: 40px; min-height: 40px; }
.dv-splitter { flex: 0 0 var(--dv-splitter-size, 4px);
	background: var(--dv-splitter-bg, #333); }
.dv-row > .dv-splitter { cursor: col-resize; }
.dv-col > .dv-splitter { cursor: row-resize; }
.dv-group { display: flex; flex-direction: column; width: 100%; height: 100%;
	background: var(--dv-group-bg, #1e1e1e); }
.dv-titlebar { flex: 0 0 auto; display: flex; align-items: center; padding: 4px 8px;
	font-weight: 600; background: var(--dv-titlebar-bg, #252526); }
.dv-title { flex: 1 1 auto; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.dv-actions { flex: 0 0 auto; display: flex; gap: 2px; }
.dv-action { cursor: pointer; border: 0; background: transparent; color: var(--dv-fg, #ddd);
	opacity: 0.55; padding: 0 5px; font: inherit; line-height: 1; }
.dv-action:hover { opacity: 1; background: var(--dv-tab-bg, #2d2d2d); }
.dv-tabstrip { flex: 0 0 auto; display: flex; overflow-x: auto;
	background: var(--dv-tabstrip-bg, #2d2d2d); }
.dv-tab { padding: 4px 12px; white-space: nowrap; cursor: pointer;
	background: var(--dv-tab-bg, #2d2d2d); border-right: 1px solid var(--dv-tab-border, #1e1e1e); }
.dv-tab.dv-active { background: var(--dv-tab-active-bg, #1e1e1e);
	color: var(--dv-tab-active-fg, #fff); }
.dv-content-slot { flex: 1 1 auto; overflow: hidden; }
.dv-overlay { position: absolute; inset: 0; pointer-events: none; }
.dv-render-overlay { position: absolute; overflow: hidden; pointer-events: auto; }
.dv-render-overlay.dv-dragging { pointer-events: none; }
.dv-drop-capture { position: fixed; inset: 0; z-index: 900; }
.dv-drop-target { position: absolute; pointer-events: none; z-index: 901; }
.dv-drop-highlight { position: absolute;
	background: var(--dv-drop-bg, rgba(80,140,255,.3)); }
.dv-watermark { display: flex; width: 100%; height: 100%;
	align-items: center; justify-content: center; opacity: 0.5; }
.dv-floating { position: absolute; z-index: 100; pointer-events: auto;
	box-shadow: 0 4px 16px rgba(0,0,0,.5); border: 1px solid var(--dv-floating-border, #444); }
.dv-resize-handle { position: absolute; right: 0; bottom: 0; width: 14px; height: 14px;
	cursor: nwse-resize; z-index: 101; background: var(--dv-resize-bg, #555); }
.dv-error-watermark { display: flex; width: 100%; height: 100%; padding: 16px;
	align-items: center; justify-content: center; text-align: center; white-space: pre-wrap;
	color: var(--dv-error-fg, #f77); }
"#;
/// Measured pixel box of each group's content slot, in **raw viewport** coords.
/// Group frames write theirs via `onmounted`/`onresize`; the content overlay localizes
/// them (`slot - root`, see [`RootOrigin`]) before positioning panels. Storing raw (not
/// container-local) is scroll/translation-robust — overlay and slots share the root's
/// frame, so a scroll or window move shifts both equally. This is the one place we
/// re-introduce measurement — dockview's `OverlayRenderContainer` (`box - box2`).
pub type GroupBoxes = Signal<HashMap<GroupId, Rect>>;

/// The dock-root div's own viewport rect, measured by [`DockArea`]. The overlay
/// subtracts its origin from each raw slot box to get container-local left/top.
pub type RootOrigin = Signal<Option<Rect>>;

impl From<dioxus::html::geometry::PixelsRect> for Rect {
	fn from(r: dioxus::html::geometry::PixelsRect) -> Self {
		Rect {
			x: r.origin.x,
			y: r.origin.y,
			width: r.size.width,
			height: r.size.height,
		}
	}
}

/// Root component. Owns the `Signal<DockModel>`, provides [`DockApi`](crate::api::DockApi)
/// + [`GroupBoxes`] via context, restores any saved layout, and stacks the three render
/// layers. `#[component]` generates the public `DockAreaProps` from these params.
///
/// - `panels`: the widgets to host; their order here is the stable render order of the
///   content overlay (independent of layout), which is what preserves instances.
/// - `storage_key`: `localStorage` key for autosave/restore; `None` disables persistence.
/// - `on_ready`: dockview's `onReady` — invoked once with the [`DockApi`] when the layout
///   starts from the *default* (a single group of all panels), so a host can script the
///   initial split layout. Skipped when a saved layout was restored, so reloads keep the
///   user's arrangement and never re-seed.
#[component]
pub fn DockArea(panels: Vec<DockPanel>, storage_key: Option<String>, on_ready: Option<Callback<DockApi>>) -> Element {
	// Restore branches three ways: absent storage → default layout; present+valid →
	// restored; present+corrupt → an error watermark (never a silent reset).
	let initial = use_hook({
		let storage_key = storage_key.clone();
		move || restore(storage_key.as_deref().and_then(crate::persist::read).as_deref())
	});
	let model = use_signal(|| match &initial {
		Restore::Loaded(m) => m.clone(),
		Restore::Default | Restore::Corrupt(_) => default_layout(&panels),
	});
	let load_error = match &initial {
		Restore::Corrupt(e) => Some(e.clone()),
		_ => None,
	};
	// Only seed via `on_ready` when we built the default layout — a restored layout is the
	// user's own arrangement and must win.
	let seed_on_ready = matches!(&initial, Restore::Default);

	let api = DockApi { model };
	use_context_provider(|| api);
	use_context_provider(|| Signal::new(HashMap::<GroupId, Rect>::new())); // GroupBoxes
	use_context_provider(|| Signal::new(None::<DragState>)); // shared drag state for tab/group DnD
	let mut root_origin: RootOrigin = use_context_provider(|| Signal::new(None));
	// Stored so `onresize` can re-measure the root's position (ResizeData carries only size).
	let mut root_handle = use_signal(|| None::<std::rc::Rc<MountedData>>);

	// Write-through autosave: re-runs whenever the model changes. localStorage `setItem` of
	// small JSON is sub-ms, so no debounce — add one only if resize-drag jank shows up.
	{
		let storage_key = storage_key.clone();
		use_effect(move || {
			let json = crate::model::serial::save(&model.read());
			if let Some(key) = storage_key.as_deref() {
				crate::persist::write(key, &json);
			}
		});
	}

	// Fire `on_ready` exactly once, after mount, on a fresh default layout. The `seeded`
	// guard holds even though seeding writes the model (which re-triggers this effect).
	{
		let mut seeded = use_signal(|| false);
		use_effect(move || {
			if seed_on_ready && !seeded() {
				seeded.set(true);
				if let Some(cb) = on_ready {
					cb.call(api);
				}
			}
		});
	}

	if let Some(message) = load_error {
		return rsx! {
			style { dangerous_inner_html: CSS }
			div { class: "dv-dockview", ErrorWatermark { message } }
		};
	}

	rsx! {
		style { dangerous_inner_html: CSS }
		div {
			class: "dv-dockview",
			onmounted: move |e| async move {
				let h = e.data();
				root_handle.set(Some(h.clone()));
				// Errs server-side / pre-hydration; overlay stays hidden until a real measure lands.
				if let Ok(rect) = h.get_client_rect().await {
					root_origin.set(Some(rect.into()));
				}
			},
			onresize: move |_| async move {
				if let Some(h) = root_handle() {
					if let Ok(rect) = h.get_client_rect().await {
						root_origin.set(Some(rect.into()));
					}
				}
			},
			grid::GridLayer {}
			div { class: "dv-overlay", content::ContentLayer { panels: panels.clone() } }
			floating::FloatingLayer {}
			drop_overlay::DropOverlay {}
		}
	}
}

/// The "stack unless positioned" default: every panel as a tab in one group (dockview's
/// behavior when `addPanel` is given no position).
pub fn default_layout(panels: &[DockPanel]) -> DockModel {
	let mut m = DockModel::default();
	let mut ids = panels.iter().map(|p| p.id.clone());
	if let Some(first) = ids.next() {
		let gid = m.mint_group_id();
		let mut group = Group::new(gid, first);
		for id in ids {
			group.insert_tab(id, group.tabs.len());
		}
		group.active = 0; // `insert_tab` activates the last inserted; the default shows the first.
		m.grid = Some(GridNode::Leaf(group));
		m.active_group = Some(gid);
	}
	for p in panels {
		m.panels.insert(p.id.clone(), PanelMeta { title: p.title.clone() });
	}
	m
}
/// Outcome of reading saved layout JSON: nothing stored, a valid layout, or a corrupt
/// payload (whose message we surface rather than silently discarding the workspace).
#[derive(Clone)]
enum Restore {
	Default,
	Loaded(DockModel),
	Corrupt(String),
}

fn restore(json: Option<&str>) -> Restore {
	match json {
		None => Restore::Default,
		Some(j) => match crate::model::serial::load(j) {
			Ok(m) => Restore::Loaded(m),
			Err(e) => Restore::Corrupt(format!("{e:?}")),
		},
	}
}

/// Shown in place of the dock when a saved layout fails to parse — keeps the corrupt
/// payload visible instead of wiping it.
#[component]
fn ErrorWatermark(message: String) -> Element {
	rsx! {
		div { class: "dv-error-watermark", "Failed to load saved layout:\n{message}" }
	}
}

// Headless structure/re-render tests over a hand-built split model. Native-only:
// they drive a `VirtualDom` + `dioxus_ssr::render`, no browser, no wasm.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
	use std::cell::RefCell;

	use super::*;
	use crate::model::{
		GroupId, PanelId,
		gridview::{Child, GridNode},
		group::Group,
	};

	thread_local! {
		/// Lets a test reach the root's signal to mutate it between renders.
		static HANDLE: RefCell<Option<Signal<DockModel>>> = const { RefCell::new(None) };
	}

	fn leaf(id: u64, tabs: &[&str]) -> GridNode {
		GridNode::Leaf(Group {
			id: GroupId(id),
			tabs: tabs.iter().map(|s| PanelId((*s).into())).collect(),
			active: 0,
		})
	}

	// Branch{H, [ g1{a,b}, Branch{V, [ g2{c}, g3{d} ]} ]}
	fn split_model() -> DockModel {
		let inner = GridNode::Branch {
			orientation: crate::geometry::Orientation::Vertical,
			children: vec![Child { node: leaf(2, &["c"]), size: 50.0 }, Child { node: leaf(3, &["d"]), size: 50.0 }],
		};
		let root = GridNode::Branch {
			orientation: crate::geometry::Orientation::Horizontal,
			children: vec![
				Child {
					node: leaf(1, &["a", "b"]),
					size: 60.0,
				},
				Child { node: inner, size: 40.0 },
			],
		};
		let mut m = DockModel::default();
		m.grid = Some(root);
		m.active_group = Some(GroupId(1));
		for (id, title) in [("a", "Alpha"), ("b", "Beta"), ("c", "Gamma"), ("d", "Delta")] {
			m.panels.insert(PanelId(id.into()), PanelMeta { title: title.into() });
		}
		m
	}

	#[component]
	fn TestRoot() -> Element {
		let model = use_signal(split_model);
		use_context_provider(|| DockApi { model });
		use_context_provider(|| Signal::new(HashMap::<GroupId, Rect>::new())); // GroupFrame measures into this
		use_context_provider(|| Signal::new(None::<DragState>));
		use_context_provider(|| Signal::new(None::<Rect>)); // RootOrigin, read by GroupFrame's float action
		HANDLE.with(|h| *h.borrow_mut() = Some(model));
		rsx! { grid::GridLayer {} }
	}

	#[test]
	fn renders_split_structure() {
		let mut dom = VirtualDom::new(TestRoot);
		dom.rebuild_in_place();
		let html = dioxus_ssr::render(&dom);

		assert_eq!(html.matches("dv-branch").count(), 2, "one row + one col branch");
		assert!(html.contains("dv-row") && html.contains("dv-col"), "both axes present");
		assert_eq!(html.matches("dv-group").count(), 3, "three leaf groups");
		assert_eq!(html.matches("dv-splitter").count(), 2, "one gutter between each sibling pair");
		assert!(html.contains("flex-basis"), "children carry percentage sizing");
		assert_eq!(html.matches("dv-active").count(), 3, "exactly one active tab per group");
		for title in ["Alpha", "Gamma", "Delta"] {
			assert!(html.contains(title), "active titles render: {title}");
		}
	}

	#[test]
	fn rerenders_on_signal_change() {
		let mut dom = VirtualDom::new(TestRoot);
		dom.rebuild_in_place();
		let before = dioxus_ssr::render(&dom);
		assert!(before.contains("Beta"), "both g1 tabs render in the strip");

		let mut sig = HANDLE.with(|h| h.borrow().expect("root mounted"));
		dom.in_runtime(|| {
			let mut m = sig.write();
			let Some(GridNode::Branch { children, .. }) = m.grid.as_mut() else {
				panic!("root is a branch")
			};
			let GridNode::Leaf(g) = &mut children[0].node else { panic!("first child is g1") };
			g.active = 1; // Alpha -> Beta
		});
		dom.render_immediate_to_vec();
		let after = dioxus_ssr::render(&dom);

		assert_ne!(before, after, "active-tab change must re-render");
		assert_eq!(after.matches("dv-active").count(), 3, "still one active tab per group");
	}

	#[test]
	fn restore_branches_none_ok_corrupt() {
		assert!(matches!(restore(None), Restore::Default), "absent storage → default");
		let json = crate::model::serial::save(&DockModel::default());
		assert!(matches!(restore(Some(&json)), Restore::Loaded(_)), "valid JSON → loaded");
		assert!(matches!(restore(Some("not json")), Restore::Corrupt(_)), "garbage → corrupt, never a reset");
	}

	#[test]
	fn corrupt_restore_renders_error_watermark() {
		#[component]
		fn Root() -> Element {
			rsx! { ErrorWatermark { message: "boom".to_string() } }
		}
		let mut dom = VirtualDom::new(Root);
		dom.rebuild_in_place();
		let html = dioxus_ssr::render(&dom);
		assert!(html.contains("dv-error-watermark"), "watermark shown for a corrupt layout");
		assert!(html.contains("boom"), "the load error message is carried through");
	}

	#[test]
	fn absent_storage_renders_dock() {
		#[component]
		fn Root() -> Element {
			let panels = vec![DockPanel {
				id: PanelId("a".into()),
				title: "A".into(),
				content: rsx! { span { "x" } },
			}];
			rsx! { DockArea { panels, storage_key: None } }
		}
		let mut dom = VirtualDom::new(Root);
		dom.rebuild_in_place();
		let html = dioxus_ssr::render(&dom);
		assert!(html.contains("dv-group"), "no storage → the default dock renders");
		// The class name also lives in the CSS `<style>`; the message only renders in the div.
		assert!(!html.contains("Failed to load saved layout"), "no watermark without a corrupt payload");
	}
}
