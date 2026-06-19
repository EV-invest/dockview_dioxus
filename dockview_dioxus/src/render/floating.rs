//! Floating groups: detached, absolutely-positioned, draggable group frames layered
//! above the grid. Port of `dockview-core/src/dockview/floatingGroupService.ts` +
//! `overlay/overlay.ts`. They reuse [`GroupFrame`](super::group::GroupFrame); only
//! their positioning (from [`FloatingGroup::rect`](crate::model::FloatingGroup)) and a
//! titlebar move/resize differ from docked frames.
//!
//! Popout windows (`popoutWindowService.ts`) are intentionally **out of scope**:
//! multi-window rendering in Dioxus web is a large separate effort; revisit only if
//! a second monitor is a hard requirement.

use dioxus::prelude::*;

/// Render each entry of `model.floating` as a movable/resizable overlay frame.
#[component]
pub fn FloatingLayer() -> Element {
	todo!("for each FloatingGroup: absolutely-positioned GroupFrame; titlebar drag moves rect; corner resizes")
}
