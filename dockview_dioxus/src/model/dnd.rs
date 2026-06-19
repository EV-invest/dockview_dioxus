//! Drag-and-drop *logic* — the state of an in-flight drag and how a drop rewrites
//! the model. Port of `dockview-core/src/dnd/` (`droptarget.ts`, `dataTransfer.ts`)
//! restricted to what a pointer-driven Dioxus app needs. The actual pointer
//! plumbing and overlay rendering live in [`crate::render`]; this is pure given the
//! resolved hover target.
//!
//! This is the `createDragToUpdates` equivalent: it owns the rule that an edge drop
//! splits and a center drop tabs.

use crate::{
	geometry::Position,
	model::{DockModel, GroupId, Location, PanelId},
};

/// What is being dragged. A whole group drag moves all its tabs at once
/// (dockview group-vs-tab DnD).
#[derive(Clone, Debug)]
pub enum DragSource {
	Tab { panel: PanelId, from_group: GroupId },
	Group(GroupId),
}

/// Live drag state, held in a `Signal` only for the drag's duration.
#[derive(Clone, Debug)]
pub struct DragState {
	pub source: DragSource,
	/// Currently hovered (group location, zone), if any — drives the overlay.
	pub hover: Option<(Location, Position)>,
}

/// Apply a completed drop: detach `source`, then either tab it into the target
/// group ([`Position::Center`]) or split the target leaf on the given edge. Removes
/// the source group if it empties, then [`normalize`](super::gridview::normalize)s.
/// The single mutation that drives all reshaping — keep it the only writer.
pub fn apply_drop(_model: &mut DockModel, _source: DragSource, _target: &Location, _position: Position) {
	todo!("detach source; center=add tab, edge=insert_split; prune empty group; normalize")
}
