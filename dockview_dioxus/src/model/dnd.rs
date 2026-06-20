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
	model::{
		DockModel, GroupId, Location, PanelId,
		gridview::{GridNode, insert_split, normalize, remove_leaf},
		group::Group,
	},
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
pub fn apply_drop(model: &mut DockModel, source: DragSource, target: &Location, position: Position) {
	let grid = model.grid.as_ref().expect("apply_drop: drop requires a grid");

	// Crux: resolve the target group's *id* before detaching the source, because
	// detaching shifts locations. We re-`locate` by id afterwards.
	let target_id = match grid.at(target).expect("apply_drop: target location must exist") {
		GridNode::Leaf(g) => g.id,
		GridNode::Branch { .. } => panic!("apply_drop: target must be a leaf group"),
	};

	let source_loc = match &source {
		DragSource::Tab { from_group, .. } => grid.locate(*from_group),
		DragSource::Group(id) => grid.locate(*id),
	}
	.expect("apply_drop: source group must exist");

	// The panel(s) being re-homed, captured before the source is touched.
	let panels: Vec<PanelId> = match &source {
		DragSource::Tab { panel, .. } => vec![panel.clone()],
		DragSource::Group(_) => {
			let GridNode::Leaf(g) = grid.at(&source_loc).expect("source leaf") else { unreachable!() };
			g.tabs.clone()
		}
	};

	// Detach the source, pruning its leaf if it empties.
	match &source {
		DragSource::Tab { panel, .. } => {
			let GridNode::Leaf(g) = model.grid.as_mut().unwrap().at_mut(&source_loc).unwrap() else { unreachable!() };
			if g.remove_tab(panel) {
				prune_leaf(model, &source_loc);
			}
		}
		DragSource::Group(_) => prune_leaf(model, &source_loc),
	}

	// Re-locate the target by id (its location may have shifted) and re-home the panels.
	let target_loc = model
		.grid
		.as_ref()
		.expect("apply_drop: grid must be non-empty after detaching")
		.locate(target_id)
		.expect("apply_drop: target group must survive the detach");

	match position {
		Position::Center => {
			let GridNode::Leaf(g) = model.grid.as_mut().unwrap().at_mut(&target_loc).unwrap() else { unreachable!() };
			for p in panels {
				let idx = g.tabs.len();
				g.insert_tab(p, idx);
			}
		}
		edge => {
			let gid = model.mint_group_id();
			let mut group = Group::new(gid, panels[0].clone());
			for p in panels.into_iter().skip(1) {
				let idx = group.tabs.len();
				group.insert_tab(p, idx);
			}
			insert_split(model.grid.as_mut().unwrap(), &target_loc, edge, group);
		}
	}

	normalize(model.grid.as_mut().unwrap());
}

/// Remove the leaf at `loc` from the grid; an emptied root collapses the whole grid.
fn prune_leaf(model: &mut DockModel, loc: &Location) {
	if loc.is_empty() {
		model.grid = None;
	} else {
		remove_leaf(model.grid.as_mut().unwrap(), loc);
	}
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use super::*;
	use crate::{
		geometry::Orientation,
		model::gridview::{Child, assert_invariants},
	};

	fn p(id: u64) -> PanelId {
		PanelId(format!("p{id}"))
	}
	fn leaf(id: u64) -> GridNode {
		GridNode::Leaf(Group::new(GroupId(id), p(id)))
	}
	fn branch(orientation: Orientation, kids: Vec<GridNode>) -> GridNode {
		let size = 100.0 / kids.len() as f64;
		GridNode::Branch { orientation, children: kids.into_iter().map(|node| Child { node, size }).collect() }
	}
	fn model(grid: GridNode, next_group_id: u64) -> DockModel {
		DockModel { grid: Some(grid), floating: vec![], maximized: None, active_group: None, next_group_id, panels: HashMap::new() }
	}

	#[test]
	fn centre_drop_tabs_into_target_and_collapses() {
		let mut m = model(branch(Orientation::Horizontal, vec![leaf(1), leaf(2)]), 3);
		apply_drop(&mut m, DragSource::Group(GroupId(2)), &vec![0], Position::Center);
		let grid = m.grid.as_ref().unwrap();
		assert_invariants(grid);
		// group 2 pruned, p2 tabbed into group 1, tree collapses to a single leaf.
		let GridNode::Leaf(g) = grid else { panic!("expected a single leaf") };
		assert_eq!(g.id, GroupId(1));
		assert_eq!(g.tabs, vec![p(1), p(2)]);
	}

	#[test]
	fn edge_drop_splits_target() {
		let mut m = model(branch(Orientation::Horizontal, vec![leaf(1), leaf(2)]), 3);
		apply_drop(&mut m, DragSource::Tab { panel: p(2), from_group: GroupId(2) }, &vec![0], Position::Bottom);
		let grid = m.grid.as_ref().unwrap();
		assert_invariants(grid);
		// group 2 emptied & pruned; p2 lives in a freshly-minted group split below leaf 1.
		let GridNode::Branch { orientation, children } = grid else { panic!("expected a branch") };
		assert_eq!(*orientation, Orientation::Vertical);
		assert_eq!(children.len(), 2);
		let GridNode::Leaf(top) = &children[0].node else { panic!() };
		let GridNode::Leaf(bottom) = &children[1].node else { panic!() };
		assert_eq!(top.id, GroupId(1));
		assert_eq!(bottom.tabs, vec![p(2)]);
		assert_eq!(bottom.id, GroupId(3)); // minted from next_group_id
	}

	#[test]
	fn three_deep_stays_alternating_after_remove() {
		let inner = branch(Orientation::Vertical, vec![leaf(2), leaf(3)]);
		let mut m = model(branch(Orientation::Horizontal, vec![leaf(1), inner]), 4);
		// move group 3 away → inner V-branch drops to one child and must collapse.
		apply_drop(&mut m, DragSource::Group(GroupId(3)), &vec![0], Position::Center);
		let grid = m.grid.as_ref().unwrap();
		assert_invariants(grid); // includes "no same-orientation parent/child"
		let GridNode::Branch { children, .. } = grid else { panic!("expected a branch") };
		assert_eq!(children.len(), 2);
		assert!(matches!(&children[1].node, GridNode::Leaf(g) if g.id == GroupId(2)));
	}
}
