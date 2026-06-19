//! The live drop indicator shown during a drag: the highlighted half/edge of the
//! hovered pane (or full-pane for a center/tab drop). Port of
//! `dockview-core/src/dnd/dropOverlay.ts`.
//!
//! Reads the current [`DragState::hover`](crate::model::dnd::DragState) and the
//! hovered group's box from [`GroupBoxes`](super::GroupBoxes), then draws one
//! absolutely-positioned highlight. The zone itself is resolved in
//! [`geometry::quadrant_at`](crate::geometry::quadrant_at) by the group's drop target.

use dioxus::prelude::*;

/// Draw the drop highlight for the active drag, or nothing when idle.
#[component]
pub fn DropOverlay() -> Element {
	todo!("if DragState.hover: draw edge/center highlight over the hovered group's box")
}
