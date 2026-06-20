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
	model::{DockModel, GroupId, PanelMeta, group::Group, gridview::GridNode},
	panel::DockPanel,
};

/// Measured pixel box of each group's content slot, in container-local coords.
/// Group frames write theirs via `onmounted`/resize; the content overlay reads it to
/// position panels. This is the one place we re-introduce measurement — exactly what
/// dockview's `OverlayRenderContainer` does (`getDomNodePagePosition` + rAF).
pub type GroupBoxes = Signal<HashMap<GroupId, Rect>>;

/// Root component. Owns the `Signal<DockModel>`, provides [`DockApi`](crate::api::DockApi)
/// + [`GroupBoxes`] via context, restores any saved layout, and stacks the three render
/// layers. `#[component]` generates the public `DockAreaProps` from these params.
///
/// - `panels`: the widgets to host; their order here is the stable render order of the
///   content overlay (independent of layout), which is what preserves instances.
/// - `storage_key`: `localStorage` key for autosave/restore; `None` disables persistence.
#[component]
pub fn DockArea(panels: Vec<DockPanel>, storage_key: Option<String>) -> Element {
	let model = use_signal(|| restore_or_default(&panels, storage_key.as_deref()));
	use_context_provider(|| DockApi { model });
	// Phase 3-5 plug in here: GroupBoxes provider + ContentLayer (content overlay),
	// FloatingLayer, DropOverlay, and the persist-on-change `use_effect`.
	rsx! {
		style { dangerous_inner_html: CSS }
		div { class: "dv-dockview", grid::GridLayer {} }
	}
}

/// Build the initial model: restore from storage if present and valid, else a
/// single-group layout holding every panel as a tab (dockview's "stack unless
/// positioned" default).
pub fn restore_or_default(panels: &[DockPanel], storage_key: Option<&str>) -> DockModel {
	// Loud-error-on-corrupt and autosave are Phase 5; native `persist::read` returns
	// `None`, so this restore branch is a no-op off-wasm.
	if let Some(json) = storage_key.and_then(crate::persist::read) {
		if let Ok(model) = crate::model::serial::load(&json) {
			return model;
		}
	}

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
.dv-titlebar { flex: 0 0 auto; padding: 4px 8px; font-weight: 600;
	background: var(--dv-titlebar-bg, #252526); }
.dv-tabstrip { flex: 0 0 auto; display: flex; overflow-x: auto;
	background: var(--dv-tabstrip-bg, #2d2d2d); }
.dv-tab { padding: 4px 12px; white-space: nowrap; cursor: pointer;
	background: var(--dv-tab-bg, #2d2d2d); border-right: 1px solid var(--dv-tab-border, #1e1e1e); }
.dv-tab.dv-active { background: var(--dv-tab-active-bg, #1e1e1e);
	color: var(--dv-tab-active-fg, #fff); }
.dv-content-slot { flex: 1 1 auto; overflow: hidden; }
.dv-watermark { display: flex; width: 100%; height: 100%;
	align-items: center; justify-content: center; opacity: 0.5; }
"#;

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
		GridNode::Leaf(Group { id: GroupId(id), tabs: tabs.iter().map(|s| PanelId((*s).into())).collect(), active: 0 })
	}

	// Branch{H, [ g1{a,b}, Branch{V, [ g2{c}, g3{d} ]} ]}
	fn split_model() -> DockModel {
		let inner = GridNode::Branch {
			orientation: crate::geometry::Orientation::Vertical,
			children: vec![Child { node: leaf(2, &["c"]), size: 50.0 }, Child { node: leaf(3, &["d"]), size: 50.0 }],
		};
		let root = GridNode::Branch {
			orientation: crate::geometry::Orientation::Horizontal,
			children: vec![Child { node: leaf(1, &["a", "b"]), size: 60.0 }, Child { node: inner, size: 40.0 }],
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
			let Some(GridNode::Branch { children, .. }) = m.grid.as_mut() else { panic!("root is a branch") };
			let GridNode::Leaf(g) = &mut children[0].node else { panic!("first child is g1") };
			g.active = 1; // Alpha -> Beta
		});
		dom.render_immediate_to_vec();
		let after = dioxus_ssr::render(&dom);

		assert_ne!(before, after, "active-tab change must re-render");
		assert_eq!(after.matches("dv-active").count(), 3, "still one active tab per group");
	}
}
