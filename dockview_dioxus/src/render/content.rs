//! The content overlay — the architectural keystone. Direct analogue of
//! `dockview-core/src/overlay/overlayRenderContainer.ts`.
//!
//! Renders one absolutely-positioned wrapper **per panel**, in a stable, id-keyed
//! list whose order is independent of the layout. Each wrapper is positioned over its
//! group's content slot using the measured [`GroupBoxes`](super::GroupBoxes). Because
//! the key and list position never change when the layout restructures, Dioxus keeps
//! each panel's component instance mounted — only the inline `style` (rect) and
//! `display` (active-tab/visible) change. This is what lets a panel hold live JS state
//! (e.g. a Google Map) while being dragged across the grid.
//!
//! Inactive-tab and off-screen panels render `display:none` (dockview's `'always'`
//! renderer) so their state survives, rather than unmounting.

use std::collections::HashMap;

use dioxus::prelude::*;

use super::{GroupBoxes, RootOrigin};
use crate::{
	api::DockApi,
	math::Rect,
	model::{GroupId, PanelId, dnd::DragState},
	panel::DockPanel,
};

/// Paint every panel's content into the flat overlay layer.
#[component]
pub fn ContentLayer(panels: Vec<DockPanel>) -> Element {
	let api = use_context::<DockApi>();
	let boxes = use_context::<GroupBoxes>();
	let root_origin = use_context::<RootOrigin>();
	let drag = use_context::<Signal<Option<DragState>>>();

	// PanelId -> (hosting group, is its active tab), from the docked grid only.
	let host: HashMap<PanelId, (GroupId, bool)> = {
		let model = api.model.read();
		let mut map = HashMap::new();
		if let Some(grid) = model.grid.as_ref() {
			for (_, group) in grid.leaves() {
				let active = group.active_panel();
				for id in &group.tabs {
					map.insert(id.clone(), (group.id, id == active));
				}
			}
		}
		map
	};

	let boxes = boxes.read();
	let origin = root_origin.read();
	// While a drag is in flight, kill pointer events on content so a panel child (e.g.
	// a map/iframe) can't swallow the drag — dockview's `disableIframePointEvents`.
	let class = if drag.read().is_some() { "dv-render-overlay dv-dragging" } else { "dv-render-overlay" };
	rsx! {
		for panel in panels.iter() {
			div {
				key: "{panel.id.0}",
				class,
				style: slot_style(host.get(&panel.id), &boxes, origin.as_ref()),
				{panel.content.clone()}
			}
		}
	}
}

/// The three visibility states (the panel is always mounted): active-and-measured ⇒
/// positioned/visible; active-but-unmeasured ⇒ `visibility:hidden` (no 0,0 flash);
/// inactive or not in the grid ⇒ `display:none`.
fn slot_style(host: Option<&(GroupId, bool)>, boxes: &HashMap<GroupId, Rect>, origin: Option<&Rect>) -> String {
	match host {
		Some((gid, true)) => match (boxes.get(gid), origin) {
			(Some(slot), Some(root)) => format!(
				"left:{}px; top:{}px; width:{}px; height:{}px;",
				slot.x - root.x,
				slot.y - root.y,
				slot.width,
				slot.height
			),
			_ => "visibility:hidden;".into(),
		},
		_ => "display:none;".into(),
	}
}

// Headless SSR tests: measurement is DOM-only (`get_client_rect` Errs server-side), so
// these assert the mount/visibility/order contract, not pixel positions. The providers
// start empty (no boxes, no origin) — exactly the pre-measure state.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
	use std::cell::RefCell;

	use super::*;
	use crate::model::{
		DockModel, PanelMeta,
		gridview::{Child, GridNode},
		group::Group,
	};

	thread_local! {
		static MODEL: RefCell<Option<DockModel>> = const { RefCell::new(None) };
		static IDS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
		static SIG: RefCell<Option<Signal<DockModel>>> = const { RefCell::new(None) };
		static DRAG: RefCell<Option<Signal<Option<DragState>>>> = const { RefCell::new(None) };
	}

	fn leaf(id: u64, tabs: &[&str], active: usize) -> GridNode {
		GridNode::Leaf(Group { id: GroupId(id), tabs: tabs.iter().map(|s| PanelId((*s).into())).collect(), active })
	}

	fn model_with(grid: GridNode, ids: &[&str]) -> DockModel {
		let mut m = DockModel::default();
		m.grid = Some(grid);
		for id in ids {
			m.panels.insert(PanelId((*id).into()), PanelMeta { title: id.to_uppercase() });
		}
		m
	}

	#[component]
	fn TestRoot() -> Element {
		let model = use_signal(|| MODEL.with(|m| m.borrow().clone().expect("model set")));
		use_context_provider(|| DockApi { model });
		use_context_provider(|| Signal::new(HashMap::<GroupId, Rect>::new()));
		use_context_provider(|| Signal::new(None::<Rect>));
		let drag = use_context_provider(|| Signal::new(None::<DragState>));
		SIG.with(|s| *s.borrow_mut() = Some(model));
		DRAG.with(|d| *d.borrow_mut() = Some(drag));
		let panels: Vec<DockPanel> = IDS.with(|ids| {
			ids.borrow()
				.iter()
				.map(|id| DockPanel { id: PanelId((*id).into()), title: id.to_string(), content: rsx! { span { "content-{id}" } } })
				.collect()
		});
		rsx! { ContentLayer { panels } }
	}

	fn render() -> String {
		let mut dom = VirtualDom::new(TestRoot);
		dom.rebuild_in_place();
		dioxus_ssr::render(&dom)
	}

	// `dv-render-overlay` is the class token of every wrapper; splitting on it yields one
	// chunk per panel (in render order), each holding that wrapper's style + content.
	fn chunks(html: &str) -> Vec<String> {
		html.split("dv-render-overlay").map(String::from).collect()
	}

	#[test]
	fn all_panels_mount_hidden_pre_measure() {
		let inner = GridNode::Branch {
			orientation: crate::geometry::Orientation::Vertical,
			children: vec![Child { node: leaf(2, &["c"], 0), size: 50.0 }, Child { node: leaf(3, &["d"], 0), size: 50.0 }],
		};
		let grid = GridNode::Branch {
			orientation: crate::geometry::Orientation::Horizontal,
			children: vec![Child { node: leaf(1, &["a", "b"], 0), size: 60.0 }, Child { node: inner, size: 40.0 }],
		};
		MODEL.with(|m| *m.borrow_mut() = Some(model_with(grid, &["a", "b", "c", "d"])));
		IDS.with(|i| *i.borrow_mut() = vec!["a", "b", "c", "d"]);
		let html = render();

		assert_eq!(html.matches("dv-render-overlay").count(), 4, "one wrapper per panel, all mounted");
		for id in ["a", "b", "c", "d"] {
			assert!(html.contains(&format!("content-{id}")), "panel {id} instance exists");
		}
		assert!(!html.contains("left:"), "nothing positioned without measured boxes");
	}

	#[test]
	fn active_inactive_flags_swap() {
		MODEL.with(|m| *m.borrow_mut() = Some(model_with(leaf(1, &["a", "b"], 0), &["a", "b"])));
		IDS.with(|i| *i.borrow_mut() = vec!["a", "b"]);

		let mut dom = VirtualDom::new(TestRoot);
		dom.rebuild_in_place();
		let parts = chunks(&dioxus_ssr::render(&dom));
		assert!(parts[1].contains("content-a") && parts[1].contains("visibility:hidden"), "active-unmeasured a is hidden, not none");
		assert!(parts[2].contains("content-b") && parts[2].contains("display:none"), "inactive b is display:none");

		let mut sig = SIG.with(|s| s.borrow().expect("root mounted"));
		dom.in_runtime(|| {
			let mut m = sig.write();
			let Some(GridNode::Leaf(g)) = m.grid.as_mut() else { panic!("single leaf") };
			g.active = 1; // a -> b
		});
		dom.render_immediate_to_vec();
		let parts = chunks(&dioxus_ssr::render(&dom));
		assert!(parts[1].contains("content-a") && parts[1].contains("display:none"), "a now inactive");
		assert!(parts[2].contains("content-b") && parts[2].contains("visibility:hidden"), "b now active");
	}

	#[test]
	fn drag_dims_content() {
		MODEL.with(|m| *m.borrow_mut() = Some(model_with(leaf(1, &["a", "b"], 0), &["a", "b"])));
		IDS.with(|i| *i.borrow_mut() = vec!["a", "b"]);

		let mut dom = VirtualDom::new(TestRoot);
		dom.rebuild_in_place();
		let html = dioxus_ssr::render(&dom);
		assert_eq!(html.matches("dv-dragging").count(), 0, "no dimming when idle");

		let mut drag = DRAG.with(|d| d.borrow().expect("mounted"));
		dom.in_runtime(|| {
			drag.set(Some(DragState { source: crate::model::dnd::DragSource::Group(GroupId(1)), hover: None }));
		});
		dom.render_immediate_to_vec();
		let html = dioxus_ssr::render(&dom);
		assert_eq!(html.matches("dv-dragging").count(), 2, "both wrappers dim during a drag");
	}

	#[test]
	fn order_follows_panels_not_layout() {
		// Grid tab order is reversed vs the panels prop; overlay order must follow the prop.
		MODEL.with(|m| *m.borrow_mut() = Some(model_with(leaf(1, &["d", "c", "b", "a"], 0), &["a", "b", "c", "d"])));
		IDS.with(|i| *i.borrow_mut() = vec!["a", "b", "c", "d"]);
		let parts = chunks(&render());
		for (i, id) in ["a", "b", "c", "d"].iter().enumerate() {
			assert!(parts[i + 1].contains(&format!("content-{id}")), "wrapper {i} hosts panel {id} (prop order)");
		}
	}
}
