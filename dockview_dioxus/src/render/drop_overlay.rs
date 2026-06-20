//! The live drop indicator shown during a drag: the highlighted half/edge of the
//! hovered pane (or full-pane for a center/tab drop). Port of
//! `dockview-core/src/dnd/dropOverlay.ts`.
//!
//! While a drag is active this also owns the full-window **capture surface** (Dioxus
//! exposes no `setPointerCapture`): a `position:fixed; inset:0` div that hit-tests the
//! pointer against the raw [`GroupBoxes`](super::GroupBoxes), resolves the zone via
//! [`quadrant_at`](crate::geometry::quadrant_at) into [`DragState::hover`], and commits
//! the drop through [`apply_drop`](crate::model::dnd::apply_drop) on `pointerup`.

use dioxus::prelude::*;

use super::{GroupBoxes, RootOrigin};
use crate::{
	api::DockApi,
	geometry::{Position, quadrant_at},
	model::{
		dnd::{DragState, apply_drop},
		gridview::GridNode,
	},
};

/// Every leaf accepts all five zones (we have no per-panel drop policy).
const ALL: &[Position] = &[Position::Top, Position::Bottom, Position::Left, Position::Right, Position::Center];

/// Draw the drop highlight + capture surface for the active drag, or nothing when idle.
#[component]
pub fn DropOverlay() -> Element {
	let mut api = use_context::<DockApi>();
	let mut drag = use_context::<Signal<Option<DragState>>>();
	let boxes = use_context::<GroupBoxes>();
	let root_origin = use_context::<RootOrigin>();

	let Some(state) = drag.read().clone() else {
		return rsx! {};
	};

	// The highlight box, localized to the dock root and shaped per zone.
	let highlight = state.hover.as_ref().and_then(|(loc, pos)| {
		let model = api.model.read();
		let GridNode::Leaf(group) = model.grid.as_ref()?.at(loc)? else { return None };
		let b = boxes.read().get(&group.id).copied()?;
		let root = *root_origin.read().as_ref()?;
		let host = format!("left:{}px; top:{}px; width:{}px; height:{}px;", b.x - root.x, b.y - root.y, b.width, b.height);
		// Within the host box, the zone fills a half (edge) or the whole box (center) —
		// percentages so the host's pixel size carries the geometry. (`dropOverlay.ts:106`.)
		let zone = match pos {
			Position::Left => "left:0; top:0; width:50%; height:100%;",
			Position::Right => "left:50%; top:0; width:50%; height:100%;",
			Position::Top => "left:0; top:0; width:100%; height:50%;",
			Position::Bottom => "left:0; top:50%; width:100%; height:50%;",
			Position::Center => "inset:0;",
		};
		Some((host, zone))
	});

	rsx! {
		div {
			class: "dv-drop-capture",
			onpointermove: move |e: PointerEvent| {
				let c = e.client_coordinates();
				let hit = {
					let model = api.model.read();
					let boxes = boxes.read();
					model.grid.as_ref().and_then(|grid| {
						grid.leaves().into_iter().find_map(|(loc, group)| {
							let b = boxes.get(&group.id)?;
							let inside = c.x >= b.x && c.x <= b.x + b.width && c.y >= b.y && c.y <= b.y + b.height;
							if !inside {
								return None;
							}
							quadrant_at(ALL, c.x - b.x, c.y - b.y, b.width, b.height, 20.0).map(|pos| (loc, pos))
						})
					})
				};
				if let Some(s) = drag.write().as_mut() {
					s.hover = hit;
				}
			},
			onpointerup: move |_| {
				if let Some(state) = drag.read().clone() {
					if let Some((loc, pos)) = state.hover {
						apply_drop(&mut api.model.write(), state.source, &loc, pos);
					}
				}
				drag.set(None);
			},
			onpointercancel: move |_| drag.set(None),
		}
		if let Some((host, zone)) = highlight {
			div { class: "dv-drop-target", style: "{host}",
				div { class: "dv-drop-highlight", style: "{zone}" }
			}
		}
	}
}

// Headless render-from-state tests: pointer events can't be dispatched in SSR, so these
// only assert the highlight geometry that a seeded `DragState.hover` + boxes produce.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
	use std::{cell::RefCell, collections::HashMap};

	use super::*;
	use crate::{
		math::Rect,
		model::{DockModel, GroupId, PanelId, PanelMeta, group::Group},
	};

	thread_local! {
		static BOXES: RefCell<Option<super::GroupBoxes>> = const { RefCell::new(None) };
		static ORIGIN: RefCell<Option<super::RootOrigin>> = const { RefCell::new(None) };
		static DRAG: RefCell<Option<Signal<Option<DragState>>>> = const { RefCell::new(None) };
	}

	#[component]
	fn TestRoot() -> Element {
		let mut m = DockModel::default();
		m.grid = Some(GridNode::Leaf(Group { id: GroupId(1), tabs: vec![PanelId("a".into())], active: 0 }));
		m.panels.insert(PanelId("a".into()), PanelMeta { title: "A".into() });
		let model = use_signal(|| m);
		use_context_provider(|| DockApi { model });
		let boxes: super::GroupBoxes = use_context_provider(|| Signal::new(HashMap::<GroupId, Rect>::new()));
		let origin: super::RootOrigin = use_context_provider(|| Signal::new(None));
		let drag = use_context_provider(|| Signal::new(None::<DragState>));
		BOXES.with(|b| *b.borrow_mut() = Some(boxes));
		ORIGIN.with(|o| *o.borrow_mut() = Some(origin));
		DRAG.with(|d| *d.borrow_mut() = Some(drag));
		rsx! { DropOverlay {} }
	}

	#[test]
	fn highlight_follows_hover_and_box() {
		let mut dom = VirtualDom::new(TestRoot);
		dom.rebuild_in_place();

		let mut boxes = BOXES.with(|b| b.borrow().expect("mounted"));
		let mut origin = ORIGIN.with(|o| o.borrow().expect("mounted"));
		let mut drag = DRAG.with(|d| d.borrow().expect("mounted"));
		dom.in_runtime(|| {
			boxes.write().insert(GroupId(1), Rect { x: 0.0, y: 0.0, width: 100.0, height: 80.0 });
			origin.set(Some(Rect { x: 0.0, y: 0.0, width: 100.0, height: 80.0 }));
			drag.set(Some(DragState {
				source: crate::model::dnd::DragSource::Group(GroupId(1)),
				hover: Some((vec![], Position::Right)),
			}));
		});
		dom.render_immediate_to_vec();
		let html = dioxus_ssr::render(&dom);

		assert_eq!(html.matches("dv-drop-highlight").count(), 1, "one highlight for the hovered zone");
		assert!(html.contains("left:50%"), "Right zone draws the right half");

		// Clearing the hover drops the highlight but keeps the capture surface.
		dom.in_runtime(|| {
			if let Some(s) = drag.write().as_mut() {
				s.hover = None;
			}
		});
		dom.render_immediate_to_vec();
		let html = dioxus_ssr::render(&dom);
		assert_eq!(html.matches("dv-drop-highlight").count(), 0, "no highlight without a hover");
		assert!(html.contains("dv-drop-capture"), "capture surface stays up during the drag");
	}
}
