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
	pub fn at(&self, location: &[usize]) -> Option<&GridNode> {
		match location.split_first() {
			None => Some(self),
			Some((&i, rest)) => match self {
				GridNode::Leaf(_) => None,
				GridNode::Branch { children, .. } => children.get(i)?.node.at(rest),
			},
		}
	}

	pub fn at_mut(&mut self, location: &[usize]) -> Option<&mut GridNode> {
		match location.split_first() {
			None => Some(self),
			Some((&i, rest)) => match self {
				GridNode::Leaf(_) => None,
				GridNode::Branch { children, .. } => children.get_mut(i)?.node.at_mut(rest),
			},
		}
	}

	/// Locate the group with `id`, returning its path. Port of `getGridLocation`
	/// (we search the model instead of reading it off a DOM element).
	pub fn locate(&self, id: GroupId) -> Option<Location> {
		match self {
			GridNode::Leaf(g) => (g.id == id).then(Vec::new),
			GridNode::Branch { children, .. } => children.iter().enumerate().find_map(|(i, c)| {
				c.node.locate(id).map(|mut loc| {
					loc.insert(0, i);
					loc
				})
			}),
		}
	}

	/// Every group in tree order, with its location — drives both the skeleton
	/// render and the geometry pass.
	pub fn leaves(&self) -> Vec<(Location, &Group)> {
		let mut out = Vec::new();
		self.collect_leaves(&mut Vec::new(), &mut out);
		out
	}

	fn collect_leaves<'a>(&'a self, path: &mut Location, out: &mut Vec<(Location, &'a Group)>) {
		match self {
			GridNode::Leaf(g) => out.push((path.clone(), g)),
			GridNode::Branch { children, .. } =>
				for (i, c) in children.iter().enumerate() {
					path.push(i);
					c.node.collect_leaves(path, out);
					path.pop();
				},
		}
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
pub fn insert_split(root: &mut GridNode, target: &Location, position: Position, group: Group) {
	let axis = position.split_orientation().expect("insert_split: Center is the tab path, not a split");
	let before = matches!(position, Position::Left | Position::Top);
	let new_node = GridNode::Leaf(group);

	// Fast path: the target's parent already splits along `axis`, so insert a sibling
	// rather than nesting a redundant branch (dockview's flatten optimization).
	if let Some((&idx, parent_path)) = target.split_last() {
		if let Some(GridNode::Branch { orientation, children }) = root.at_mut(parent_path) {
			if *orientation == axis {
				let half = children[idx].size / 2.0;
				children[idx].size = half;
				let at = if before { idx } else { idx + 1 };
				children.insert(at, Child { node: new_node, size: half });
				normalize(root);
				return;
			}
		}
	}

	// Otherwise replace the target node with a fresh 2-child branch at 50/50.
	let node = root.at_mut(target).expect("insert_split: target location must exist");
	let existing = node.clone();
	let children = if before {
		vec![Child { node: new_node, size: 50.0 }, Child { node: existing, size: 50.0 }]
	} else {
		vec![Child { node: existing, size: 50.0 }, Child { node: new_node, size: 50.0 }]
	};
	*node = GridNode::Branch { orientation: axis, children };
	normalize(root);
}

/// Remove the leaf at `location`, collapsing a now-single-child branch into its
/// sibling and merging same-axis branches. Port of `Gridview.removeView` (the
/// tree-collapse half) — the trickiest mutation; get its invariants right.
pub fn remove_leaf(root: &mut GridNode, location: &Location) {
	let (&idx, parent_path) = location.split_last().expect("remove_leaf: cannot remove the root via remove_leaf");
	let parent = root.at_mut(parent_path).expect("remove_leaf: parent must exist");
	let GridNode::Branch { children, .. } = parent else {
		panic!("remove_leaf: parent of a leaf must be a branch");
	};
	let mut sizes: Vec<f64> = children.iter().map(|c| c.size).collect();
	crate::model::splitview::remove_child_size(&mut sizes, idx);
	children.remove(idx);
	for (c, s) in children.iter_mut().zip(&sizes) {
		c.size = *s;
	}
	normalize(root);
}

/// Move `delta_pct` across the splitter between children `index` and `index+1` of the
/// branch at `parent`, clamped by [`resize_pair`](super::splitview::resize_pair). The
/// single sizing-edit path shared by the splitter drag and the fuzzer.
pub fn resize_branch(root: &mut GridNode, parent: &Location, index: usize, delta_pct: f64) {
	let GridNode::Branch { children, .. } = root.at_mut(parent).expect("resize_branch: parent location must exist") else {
		panic!("resize_branch: parent must be a branch");
	};
	let mut sizes: Vec<f64> = children.iter().map(|c| c.size).collect();
	crate::model::splitview::resize_pair(&mut sizes, index, delta_pct);
	for (c, s) in children.iter_mut().zip(&sizes) {
		c.size = *s;
	}
}

/// Restore tree invariants after an edit: drop single-child branches, merge nested
/// same-orientation branches (rescaling percentages), clamp tiny sizes. Port of
/// `Gridview.normalize` generalized to the n-ary case (see also react-mosaic's
/// `normalizeMosaicTree`). Call after every mutation. Idempotent.
pub fn normalize(root: &mut GridNode) {
	if let GridNode::Branch { orientation, children } = root {
		let axis = *orientation;

		for c in children.iter_mut() {
			normalize(&mut c.node);
		}

		// (a) Splice same-orientation child branches into this one, carry-scaling each
		// grandchild's % by the child's own %, so the flattened sizes stay consistent.
		let mut i = 0;
		while i < children.len() {
			let same = matches!(&children[i].node, GridNode::Branch { orientation: o, .. } if *o == axis);
			if same {
				let child = children.remove(i);
				let pct = child.size;
				let GridNode::Branch { children: grand, .. } = child.node else { unreachable!() };
				let count = grand.len();
				for (k, mut gc) in grand.into_iter().enumerate() {
					gc.size = gc.size * pct / 100.0;
					children.insert(i + k, gc);
				}
				i += count;
			} else {
				i += 1;
			}
		}

		// (c) Renormalize to sum 100, lifting any degenerate ~0 share to the floor first.
		if children.len() >= 2 {
			for c in children.iter_mut() {
				c.size = c.size.max(crate::model::splitview::MIN_CHILD_PCT);
			}
			let sum: f64 = children.iter().map(|c| c.size).sum();
			for c in children.iter_mut() {
				c.size = c.size / sum * 100.0;
			}
		}
	}

	// (b) A branch with one child *is* that child — promote it. Done after the borrow
	// above ends so we can reassign `*root`.
	let promote = match root {
		GridNode::Branch { children, .. } if children.len() == 1 => Some(children.remove(0).node),
		_ => None,
	};
	if let Some(only) = promote {
		*root = only;
	}
}

/// The full invariant bundle a normalized tree must satisfy. Shared by the gridview
/// and dnd scenario tests so both assert the same contract.
#[cfg(test)]
pub(crate) fn assert_invariants(node: &GridNode) {
	match node {
		GridNode::Leaf(g) => {
			assert!(!g.tabs.is_empty(), "no empty group");
			assert!(g.active < g.tabs.len(), "active index in range");
		}
		GridNode::Branch { orientation, children } => {
			assert!(children.len() >= 2, "no single/empty-child branch");
			let sum: f64 = children.iter().map(|c| c.size).sum();
			assert!((sum - 100.0).abs() < 1e-6, "branch sizes sum to 100, got {sum}");
			for c in children {
				if let GridNode::Branch { orientation: o, .. } = &c.node {
					assert_ne!(o, orientation, "no same-orientation parent/child");
				}
				assert_invariants(&c.node);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::model::group::Group;

	fn leaf(id: u64) -> GridNode {
		GridNode::Leaf(Group::new(GroupId(id), crate::model::PanelId(format!("p{id}"))))
	}
	fn branch(orientation: Orientation, children: Vec<(GridNode, f64)>) -> GridNode {
		GridNode::Branch {
			orientation,
			children: children.into_iter().map(|(node, size)| Child { node, size }).collect(),
		}
	}

	fn idempotent(mut t: GridNode) {
		normalize(&mut t);
		let once = t.clone();
		normalize(&mut t);
		assert_eq!(t, once, "normalize must be idempotent");
		assert_invariants(&t);
	}

	#[test]
	fn single_child_branch_collapses() {
		idempotent(branch(Orientation::Horizontal, vec![(leaf(1), 100.0)]));
	}

	#[test]
	fn same_axis_nesting_flattens() {
		let inner = branch(Orientation::Horizontal, vec![(leaf(2), 50.0), (leaf(3), 50.0)]);
		idempotent(branch(Orientation::Horizontal, vec![(leaf(1), 50.0), (inner, 50.0)]));
	}

	#[test]
	fn off_hundred_sums_renormalize() {
		idempotent(branch(Orientation::Vertical, vec![(leaf(1), 10.0), (leaf(2), 10.0), (leaf(3), 10.0)]));
	}

	#[test]
	fn deeply_degenerate_tree() {
		// single-child branch wrapping a same-axis nest with bad sums
		let nest = branch(Orientation::Vertical, vec![(leaf(2), 3.0), (leaf(3), 7.0)]);
		let mid = branch(Orientation::Vertical, vec![(nest, 200.0)]);
		idempotent(branch(Orientation::Horizontal, vec![(leaf(1), 1.0), (mid, 9.0)]));
	}
}
