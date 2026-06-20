//! A group frame: titlebar + tab strip + an empty content slot. Port of
//! `dockview-core/src/dockview/dockviewGroupPanel*` + `tabGroup.ts`, declarative.
//!
//! Crucially the content slot stays **empty** — actual panel content is painted by
//! the [content overlay](super::content) and positioned over this slot's measured
//! box. The frame only contributes chrome and that measured box. (insilicoterminal's
//! `.titlebar` / `.subtitlebar` / `.footerbar` map onto this frame.)

use dioxus::prelude::*;

use crate::{
	api::DockApi,
	model::{
		Location,
		dnd::{DragSource, DragState},
		gridview::GridNode,
	},
};

/// Movement (px) past which a tab `pointerdown` becomes a drag rather than a click.
const DRAG_THRESHOLD: f64 = 4.0;

/// One pane: titlebar (active panel's title) + tab strip + an empty content slot.
/// Static this phase — titlebar drag, maximize/close handlers, the content slot's
/// measurement `onmounted`, and the 5-zone drop target are Phase 3–4.
#[component]
pub fn GroupFrame(location: Location) -> Element {
	let api = use_context::<DockApi>();
	let (gid, title) = {
		let model = api.model.read();
		let root = model.grid.as_ref().expect("GroupFrame rendered without a grid");
		let GridNode::Leaf(group) = root.at(&location).expect("GroupFrame: location must resolve") else {
			panic!("GroupFrame: location must point at a leaf");
		};
		(group.id, model.panels.get(group.active_panel()).expect("active panel has metadata").title.clone())
	};
	let mut boxes = use_context::<super::GroupBoxes>();
	let mut drag = use_context::<Signal<Option<DragState>>>();
	// Kept so `onresize` re-measures position (ResizeData carries only size).
	let mut handle = use_signal(|| None::<std::rc::Rc<MountedData>>);
	rsx! {
		div { class: "dv-group",
			div {
				class: "dv-titlebar",
				onpointerdown: move |e: PointerEvent| {
					if e.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
						drag.set(Some(DragState { source: DragSource::Group(gid), hover: None }));
					}
				},
				"{title}"
			}
			TabStrip { location: location.clone() }
			div {
				class: "dv-content-slot",
				onmounted: move |e| async move {
					let h = e.data();
					handle.set(Some(h.clone()));
					// Errs server-side / pre-hydration; leaving the box unset keeps the panel hidden.
					if let Ok(rect) = h.get_client_rect().await {
						boxes.write().insert(gid, rect.into());
					}
				},
				onresize: move |_| async move {
					if let Some(h) = handle() {
						if let Ok(rect) = h.get_client_rect().await {
							boxes.write().insert(gid, rect.into());
						}
					}
				},
			}
		}
	}
}

/// In-flight tab press: where it started and whether it has promoted to a drag.
/// A press under [`DRAG_THRESHOLD`] is a plain activate on `pointerup`.
#[derive(Clone, Copy)]
struct TabPress {
	index: usize,
	x: f64,
	y: f64,
	dragging: bool,
}

/// The tab strip: one tab per panel in `Group.tabs`, marking the active one. A tab
/// click activates it; dragging past [`DRAG_THRESHOLD`] promotes to a
/// [`DragState::Tab`] drag source (the global [`DropOverlay`](super::drop_overlay)
/// then owns the hover/drop).
#[component]
pub fn TabStrip(location: Location) -> Element {
	let mut api = use_context::<DockApi>();
	let mut drag = use_context::<Signal<Option<DragState>>>();
	let (gid, tabs, active) = {
		let model = api.model.read();
		let root = model.grid.as_ref().expect("TabStrip rendered without a grid");
		let GridNode::Leaf(group) = root.at(&location).expect("TabStrip: location must resolve") else {
			panic!("TabStrip: location must point at a leaf");
		};
		let tabs: Vec<(crate::model::PanelId, String)> = group
			.tabs
			.iter()
			.map(|id| (id.clone(), model.panels.get(id).expect("tab panel has metadata").title.clone()))
			.collect();
		(group.id, tabs, group.active)
	};
	let mut press = use_signal(|| None::<TabPress>);
	rsx! {
		div { class: "dv-tabstrip",
			for (i, (id, title)) in tabs.iter().enumerate() {
				div {
					key: "{id.0}",
					class: if i == active { "dv-tab dv-active" } else { "dv-tab" },
					onpointerdown: {
						move |e: PointerEvent| {
							if e.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
								let c = e.client_coordinates();
								press.set(Some(TabPress { index: i, x: c.x, y: c.y, dragging: false }));
							}
						}
					},
					onpointermove: {
						let id = id.clone();
						move |e: PointerEvent| {
							let Some(p) = press() else { return };
							if p.index != i || p.dragging {
								return;
							}
							let c = e.client_coordinates();
							if (c.x - p.x).abs() > DRAG_THRESHOLD || (c.y - p.y).abs() > DRAG_THRESHOLD {
								press.set(Some(TabPress { dragging: true, ..p }));
								drag.set(Some(DragState { source: DragSource::Tab { panel: id.clone(), from_group: gid }, hover: None }));
							}
						}
					},
					onpointerup: {
						let location = location.clone();
						move |_| {
							// A press that never crossed the threshold is a plain activate.
							if let Some(p) = press() {
								if !p.dragging {
									let mut model = api.model.write();
									let GridNode::Leaf(group) = model.grid.as_mut().expect("grid").at_mut(&location).expect("TabStrip location resolves") else {
										panic!("TabStrip: location must point at a leaf");
									};
									group.active = i;
								}
							}
							press.set(None);
						}
					},
					"{title}"
				}
			}
		}
	}
}
