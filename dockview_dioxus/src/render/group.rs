//! A group frame: titlebar + tab strip + an empty content slot. Port of
//! `dockview-core/src/dockview/dockviewGroupPanel*` + `tabGroup.ts`, declarative.
//!
//! Crucially the content slot stays **empty** — actual panel content is painted by
//! the [content overlay](super::content) and positioned over this slot's measured
//! box. The frame only contributes chrome and that measured box. (insilicoterminal's
//! `.titlebar` / `.subtitlebar` / `.footerbar` map onto this frame.)

use dioxus::prelude::*;

use crate::model::Location;

/// One pane: drag-handle titlebar, the tab strip, the (empty, measured) content slot,
/// and the 5-zone drop target that wraps it.
#[component]
pub fn GroupFrame(location: Location) -> Element {
	// - read the Group at `location` from the model
	// - titlebar: pointerdown starts a Group drag (DragSource::Group); maximize/close buttons
	// - TabStrip { location }
	// - content slot: div with `onmounted` + resize observer writing its Rect into GroupBoxes
	// - DropTarget overlay computing Position via geometry::quadrant_at, feeding DragState.hover
	todo!("render titlebar (drag/maximize/close), TabStrip, measured empty content slot, drop target")
}

/// The tab strip. Each tab: click to activate, pointerdown to start a `Tab` drag,
/// drag-within to reorder. Port of `tabGroup.ts`.
#[component]
pub fn TabStrip(location: Location) -> Element {
	todo!("render tabs from Group.tabs; activate on click; start Tab drag on pointerdown")
}
