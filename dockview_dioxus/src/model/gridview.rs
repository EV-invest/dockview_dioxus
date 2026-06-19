//! The recursive split-tree. Port of `dockview-core/src/gridview/` (`gridview.ts`,
//! `branchNode.ts`, `leafNode.ts`, `types.ts`).
//!
//! Deviations from dockview, by design:
//! - dockview derives a branch's orientation from its *depth* (root orientation,
//!   flipped at each level). We store [`Orientation`] on each [`GridNode::Branch`]
//!   explicitly: clearer in Rust, serde-clean, and [`normalize`] re-establishes the
//!   "no two nested branches share an axis" invariant after edits.
//! - children carry a percentage `size` (sum 100 per branch); pixel layout is CSS's job.
//! - `BranchNode`/`LeafNode` classes collapse into one `enum` — no DOM to own.

use crate::{
	geometry::{Orientation, Position},
	model::{GroupId, Location, group::Group},
};

/// A node in the layout tree: either a tab-group leaf or an axis split.
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
pub enum GridNode {
	/// A tab-group occupying this cell.
	Leaf(Group),
	/// An ordered split along `orientation`; `children[i].size` is a percentage.
	Branch { orientation: Orientation, children: Vec<Child> },
}
impl GridNode {
	/// Borrow the node at `location` (empty path = self). `None` if the path
	/// escapes the tree. Port of `Gridview.getNode`.
	pub fn at(&self, _location: &[usize]) -> Option<&GridNode> {
		todo!("walk child indices to the node at `location`")
	}

	pub fn at_mut(&mut self, _location: &[usize]) -> Option<&mut GridNode> {
		todo!("mutable `at`")
	}

	/// Locate the group with `id`, returning its path. Port of `getGridLocation`
	/// (we search the model instead of reading it off a DOM element).
	pub fn locate(&self, _id: GroupId) -> Option<Location> {
		todo!("DFS for the leaf carrying `id`")
	}

	/// Every group in tree order, with its location — drives both the skeleton
	/// render and the geometry pass.
	pub fn leaves(&self) -> Vec<(Location, &Group)> {
		todo!("collect (location, group) for all leaves")
	}
}

/// A sized child within a branch.
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct Child {
	pub node: GridNode,
	/// Percentage of the parent along its split axis. Branch children sum to 100.
	pub size: f64,
}

/// Insert `group` adjacent to the leaf at `target` on its `position` edge: splits
/// that leaf into a new branch (or, if the parent already runs the right axis,
/// inserts a sibling — dockview's flatten optimization). `Center` is invalid here
/// (that path adds a tab to the group instead). Port of `Gridview.addView` +
/// `getRelativeLocation`.
pub fn insert_split(_root: &mut GridNode, _target: &Location, _position: Position, _group: Group) {
	todo!("split target leaf into a branch (or insert sibling if axis matches), size 50/50")
}

/// Remove the leaf at `location`, collapsing a now-single-child branch into its
/// sibling and merging same-axis branches. Port of `Gridview.removeView` (the
/// tree-collapse half) — the trickiest mutation; get its invariants right.
pub fn remove_leaf(_root: &mut GridNode, _location: &Location) {
	todo!("remove leaf, collapse redundant branch, redistribute sizes")
}

/// Restore tree invariants after an edit: drop single-child branches, merge nested
/// same-orientation branches (rescaling percentages), clamp tiny sizes. Port of
/// `Gridview.normalize` generalized to the n-ary case (see also react-mosaic's
/// `normalizeMosaicTree`). Call after every mutation.
pub fn normalize(_root: &mut GridNode) {
	todo!("collapse/merge redundant nodes, keep each branch's sizes summing to 100")
}
