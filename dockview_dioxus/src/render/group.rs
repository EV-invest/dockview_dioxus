//! A group frame: titlebar + tab strip + an empty content slot. Port of
//! `dockview-core/src/dockview/dockviewGroupPanel*` + `tabGroup.ts`, declarative.
//!
//! Crucially the content slot stays **empty** — actual panel content is painted by
//! the [content overlay](super::content) and positioned over this slot's measured
//! box. The frame only contributes chrome and that measured box. (insilicoterminal's
//! `.titlebar` / `.subtitlebar` / `.footerbar` map onto this frame.)
//!
//! Frames are addressed by [`GroupAddr`] so a single component renders both docked
//! leaves and floating groups.

use dioxus::prelude::*;

use crate::{
	api::DockApi,
	model::{
		GroupAddr,
		dnd::{DragSource, DragState, FloatMove},
	},
};

/// Movement (px) past which a tab `pointerdown` becomes a drag rather than a click.
const DRAG_THRESHOLD: f64 = 4.0;

/// One pane: titlebar (active panel's title) + tab strip + an empty content slot.
/// The titlebar starts a drag (group move, or a floating-frame move) on press and
/// toggles maximize on double-click; the slot's `onmounted`/`onresize` feed its box
/// to the content overlay.
#[component]
pub fn GroupFrame(addr: GroupAddr) -> Element {
	let mut api = use_context::<DockApi>();
	let (gid, title) = {
		let model = api.model.read();
		let group = model.group(&addr);
		(group.id, model.panels.get(group.active_panel()).expect("active panel has metadata").title.clone())
	};
	let mut boxes = use_context::<super::GroupBoxes>();
	let root_origin = use_context::<super::RootOrigin>();
	let mut drag = use_context::<Signal<Option<DragState>>>();
	let is_max = matches!(&addr, GroupAddr::Docked(loc) if api.model.read().maximized.as_ref() == Some(loc));
	// Kept so `onresize` re-measures position (ResizeData carries only size).
	let mut handle = use_signal(|| None::<std::rc::Rc<MountedData>>);

	// A floating frame moves via its rect (a pure reposition fires no ResizeObserver), so
	// re-measure the slot whenever this float's rect changes; docked frames no-op here.
	{
		let addr = addr.clone();
		use_effect(move || {
			let GroupAddr::Floating(idx) = &addr else { return };
			let _ = api.model.read().floating.get(*idx).map(|fg| fg.rect); // subscribe to the rect
			if let Some(h) = handle() {
				spawn(async move {
					if let Ok(rect) = h.get_client_rect().await {
						boxes.write().insert(gid, rect.into());
					}
				});
			}
		});
	}

	rsx! {
		div { class: "dv-group",
			div {
				class: "dv-titlebar",
				onpointerdown: {
					let addr = addr.clone();
					move |e: PointerEvent| {
						if e.trigger_button() != Some(dioxus::html::input_data::MouseButton::Primary) {
							return;
						}
						match &addr {
							GroupAddr::Docked(_) => drag.set(Some(DragState { source: DragSource::Group(gid), hover: None, floating_move: None })),
							GroupAddr::Floating(idx) => {
								let c = e.client_coordinates();
								let rect = api.model.read().floating[*idx].rect;
								drag.set(Some(DragState {
									source: DragSource::Group(gid),
									hover: None,
									floating_move: Some(FloatMove { idx: *idx, offset_x: c.x - rect.x, offset_y: c.y - rect.y }),
								}));
							}
						}
					}
				},
				ondoubleclick: {
					let addr = addr.clone();
					move |_| {
						// Maximize is render-only; a floating frame has no grid location to maximize.
						if let GroupAddr::Docked(loc) = &addr {
							let mut m = api.model.write();
							m.maximized = m.maximized.is_none().then(|| loc.clone());
						}
					}
				},
				span { class: "dv-title", "{title}" }
				if let GroupAddr::Docked(loc) = &addr {
					div {
						class: "dv-actions",
						// Keep clicks here from starting a group drag on the titlebar.
						onpointerdown: |e: PointerEvent| e.stop_propagation(),
						button {
							class: "dv-action",
							title: "Float group",
							onclick: move |_| {
								// Float in place: the group's measured box, localized to the dock root.
								let root = root_origin().unwrap_or_default();
								let rect = match boxes.read().get(&gid).copied() {
									Some(b) => crate::math::Rect { x: b.x - root.x, y: b.y - root.y, width: b.width, height: b.height },
									None => crate::math::Rect { x: 80.0, y: 80.0, width: 320.0, height: 220.0 },
								};
								api.float(gid, rect);
							},
							"⧉"
						}
						button {
							class: "dv-action",
							title: if is_max { "Restore" } else { "Maximize" },
							onclick: {
								let loc = loc.clone();
								move |_| {
									let mut m = api.model.write();
									m.maximized = (m.maximized.as_ref() != Some(&loc)).then(|| loc.clone());
								}
							},
							if is_max { "⤡" } else { "⤢" }
						}
					}
				}
			}
			TabStrip { addr: addr.clone() }
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

/// The tab strip: one tab per panel in `Group.tabs`, marking the active one. A tab
/// click activates it; dragging past [`DRAG_THRESHOLD`] promotes to a
/// [`DragState::Tab`] drag source (the global [`DropOverlay`](super::drop_overlay)
/// then owns the hover/drop). Works for docked and floating groups alike.
#[component]
pub fn TabStrip(addr: GroupAddr) -> Element {
	let mut api = use_context::<DockApi>();
	let mut drag = use_context::<Signal<Option<DragState>>>();
	let (gid, tabs, active) = {
		let model = api.model.read();
		let group = model.group(&addr);
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
								drag.set(Some(DragState { source: DragSource::Tab { panel: id.clone(), from_group: gid }, hover: None, floating_move: None }));
							}
						}
					},
					onpointerup: {
						let addr = addr.clone();
						move |_| {
							// A press that never crossed the threshold is a plain activate.
							if let Some(p) = press() {
								if !p.dragging {
									api.model.write().group_mut(&addr).active = i;
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
/// In-flight tab press: where it started and whether it has promoted to a drag.
/// A press under [`DRAG_THRESHOLD`] is a plain activate on `pointerup`.
#[derive(Clone, Copy)]
struct TabPress {
	index: usize,
	x: f64,
	y: f64,
	dragging: bool,
}
