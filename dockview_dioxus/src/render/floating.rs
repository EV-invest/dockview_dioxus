//! Floating groups: detached, absolutely-positioned, draggable group frames layered
//! above the grid. Port of `dockview-core/src/dockview/floatingGroupService.ts` +
//! `overlay/overlay.ts`. They reuse [`GroupFrame`](super::group::GroupFrame); only
//! their positioning (from [`FloatingGroup::rect`](crate::model::FloatingGroup)), the
//! titlebar move (owned by [`DropOverlay`](super::drop_overlay), so a drop can re-dock),
//! and a corner resize differ from docked frames.
//!
//! Popout windows (`popoutWindowService.ts`) are intentionally **out of scope**:
//! multi-window rendering in Dioxus web is a large separate effort; revisit only if
//! a second monitor is a hard requirement.

use dioxus::prelude::*;

use super::group::GroupFrame;
use crate::{api::DockApi, model::GroupAddr};

/// Minimum floating-frame size (px) the corner resize respects.
const MIN_W: f64 = 120.0;
const MIN_H: f64 = 80.0;
/// Render each entry of `model.floating` as a movable/resizable overlay frame, keyed by
/// group id so reorders don't remount.
#[component]
pub fn FloatingLayer() -> Element {
	let api = use_context::<DockApi>();
	let ids: Vec<u64> = api.model.read().floating.iter().map(|fg| fg.group.id.0).collect();
	rsx! {
		for (idx, id) in ids.iter().enumerate() {
			FloatingFrame { key: "{id}", idx }
		}
	}
}

/// Corner-resize gesture captured at `pointerdown`: pointer start + the rect's size then.
#[derive(Clone, Copy)]
struct ResizeStart {
	px: f64,
	py: f64,
	w: f64,
	h: f64,
}

/// One floating frame: an absolutely-positioned [`GroupFrame`] plus a bottom-right resize
/// handle. The titlebar move lives in the frame + drop overlay (so it can re-dock); this
/// owns only the local corner-resize capture (the Splitter idiom).
#[component]
fn FloatingFrame(idx: usize) -> Element {
	let mut api = use_context::<DockApi>();
	let rect = api.model.read().floating[idx].rect;
	let mut resize = use_signal(|| None::<ResizeStart>);
	rsx! {
		div {
			class: "dv-floating",
			style: "left:{rect.x}px; top:{rect.y}px; width:{rect.width}px; height:{rect.height}px;",
			GroupFrame { addr: GroupAddr::Floating(idx) }
			div {
				class: "dv-resize-handle",
				onpointerdown: move |e: PointerEvent| {
					if e.trigger_button() != Some(dioxus::html::input_data::MouseButton::Primary) {
						return;
					}
					e.stop_propagation();
					let c = e.client_coordinates();
					resize.set(Some(ResizeStart { px: c.x, py: c.y, w: rect.width, h: rect.height }));
				},
			}
			if resize().is_some() {
				div {
					style: "position:fixed; inset:0; z-index:1000; cursor:nwse-resize;",
					onpointermove: move |e: PointerEvent| {
						let Some(s) = resize() else { return };
						let c = e.client_coordinates();
						let mut m = api.model.write();
						let r = &mut m.floating[idx].rect;
						r.width = (s.w + c.x - s.px).max(MIN_W);
						r.height = (s.h + c.y - s.py).max(MIN_H);
					},
					onpointerup: move |_| resize.set(None),
					onpointercancel: move |_| resize.set(None),
				}
			}
		}
	}
}
