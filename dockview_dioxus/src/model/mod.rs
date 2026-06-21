//! The pure, serializable layout model — the single source of truth held in one
//! `Signal<DockModel>`. No DOM, no Dioxus: just data + tree operations, so it is
//! unit-testable in isolation and `cargo check`s on any target.
//!
//! Layer map against `docs/refs/dockview-core/src`:
//! - [`splitview`] ← `splitview/` — 1-D proportional sizing math (we keep only the
//!   resize/distribute math; actual pixel layout is delegated to CSS flexbox).
//! - [`gridview`] ← `gridview/` — the recursive branch/leaf tree + location paths,
//!   add/remove/move/normalize, maximize, serialization.
//! - [`group`]    ← `dockview/dockviewGroupPanel*` — a leaf's tab-group (many panels, one active).
//! - [`dnd`]      ← `dnd/` — drag state + drop→tree mutation (the `createDragToUpdates` equivalent).
//! - [`serial`]   ← `gridview.ts`/`deserializer.ts` — JSON shapes + (de)serialization.

pub mod dnd;
pub mod gridview;
pub mod group;
pub mod serial;
pub mod splitview;

use std::collections::HashMap;

pub use dnd::DragSource;
pub use gridview::{Child, GridNode};
pub use group::Group;

use crate::{geometry::Position, math::Rect};

/// Path of child indices from the grid root to a node (dockview `location: number[]`).
pub type Location = Vec<usize>;

/// Where a renderable group lives. The grid addresses leaves by [`Location`]; floating
/// groups have none, so render code that must handle both (frames, tab strips) is keyed
/// by this instead. Identity stays [`GroupId`] — this only carries *where + kind*.
#[derive(Clone, Debug, PartialEq)]
pub enum GroupAddr {
	Docked(Location),
	Floating(usize),
}
/// Stable identity of a panel (a single widget). Provided by the consumer; used as
/// the render key that keeps a panel's component instance alive across restructuring.
#[derive(Clone, Debug, serde::Deserialize, Eq, Hash, PartialEq, serde::Serialize)]
pub struct PanelId(pub String);

/// Stable identity of a group (a tab-strip leaf holding 1+ panels).
#[derive(Clone, Copy, Debug, serde::Deserialize, Eq, Hash, PartialEq, serde::Serialize)]
pub struct GroupId(pub u64);

/// A detached, absolutely-positioned group floating above the grid. Floating and
/// [`maximized`](DockModel::maximized) are overlay state *beside* the tree, never
/// nodes within it — matching dockview (`floatingGroupService`, gridview maximize).
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct FloatingGroup {
	pub group: Group,
	pub rect: crate::math::Rect,
}

/// The complete layout state. One of these lives in a `Signal`; everything renders
/// from it and every interaction is a pure mutation of it.
#[derive(Clone, Debug, Default, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct DockModel {
	/// The docked split-tree. `None` only while empty (dockview shows a watermark).
	pub grid: Option<GridNode>,
	pub floating: Vec<FloatingGroup>,
	/// Location of the single maximized leaf, if any (render-time flag, not a tree edit).
	pub maximized: Option<Location>,
	pub active_group: Option<GroupId>,
	/// Monotonic source for [`GroupId`]s. Avoids `Math.random`-style collisions.
	next_group_id: u64,
	/// Per-panel metadata (title, etc.) kept out of the tree so the tree stays small —
	/// dockview's separation of the grid from its top-level `panels` map.
	pub panels: HashMap<PanelId, PanelMeta>,
}
impl DockModel {
	pub fn mint_group_id(&mut self) -> GroupId {
		let id = GroupId(self.next_group_id);
		self.next_group_id += 1;
		id
	}

	/// Borrow the group at `addr` (docked leaf or floating entry).
	pub(crate) fn group(&self, addr: &GroupAddr) -> &Group {
		match addr {
			GroupAddr::Docked(loc) => {
				let GridNode::Leaf(g) = self.grid.as_ref().expect("group: docked addr needs a grid").at(loc).expect("group: location resolves") else {
					panic!("group: docked addr must point at a leaf");
				};
				g
			}
			GroupAddr::Floating(i) => &self.floating[*i].group,
		}
	}

	pub(crate) fn group_mut(&mut self, addr: &GroupAddr) -> &mut Group {
		match addr {
			GroupAddr::Docked(loc) => {
				let GridNode::Leaf(g) = self
					.grid
					.as_mut()
					.expect("group_mut: docked addr needs a grid")
					.at_mut(loc)
					.expect("group_mut: location resolves")
				else {
					panic!("group_mut: docked addr must point at a leaf");
				};
				g
			}
			GroupAddr::Floating(i) => &mut self.floating[*i].group,
		}
	}

	/// Dock a panel relative to an existing group (or as the first panel when the grid is
	/// empty). Core entry point — `DockviewApi.addPanel`.
	pub fn add_panel(&mut self, panel: PanelId, title: String, target: Option<(Location, Position)>) {
		self.panels.insert(panel.clone(), PanelMeta { title });

		if self.grid.is_none() {
			let gid = self.mint_group_id();
			self.grid = Some(GridNode::Leaf(Group::new(gid, panel)));
			self.active_group = Some(gid);
			return;
		}

		match target {
			Some((loc, Position::Center)) => {
				let g = self.group_mut(&GroupAddr::Docked(loc));
				let idx = g.tabs.len();
				g.insert_tab(panel, idx);
			}
			Some((loc, edge)) => {
				let gid = self.mint_group_id();
				gridview::insert_split(self.grid.as_mut().unwrap(), &loc, edge, Group::new(gid, panel));
				self.maximized = None; // the split shifts locations, staling any maximized one.
			}
			None => {
				// Tab into the active group, or the first leaf if there is none.
				let loc = self
					.active_group
					.and_then(|gid| self.grid.as_ref().unwrap().locate(gid))
					.or_else(|| self.grid.as_ref().unwrap().leaves().first().map(|(l, _)| l.clone()))
					.expect("add_panel: a non-empty grid has at least one leaf");
				let g = self.group_mut(&GroupAddr::Docked(loc));
				let idx = g.tabs.len();
				g.insert_tab(panel, idx);
			}
		}
	}

	pub fn move_panel(&mut self, panel: PanelId, target: Location, position: Position) {
		let from_group = self
			.grid
			.as_ref()
			.expect("move_panel: needs a grid")
			.leaves()
			.into_iter()
			.find(|(_, g)| g.tabs.contains(&panel))
			.map(|(_, g)| g.id)
			.expect("move_panel: panel must already be docked");
		dnd::apply_drop(self, DragSource::Tab { panel, from_group }, &target, position);
	}

	pub fn remove_panel(&mut self, panel: PanelId) {
		let (loc, gid) = self
			.grid
			.as_ref()
			.expect("remove_panel: needs a grid")
			.leaves()
			.into_iter()
			.find(|(_, g)| g.tabs.contains(&panel))
			.map(|(l, g)| (l, g.id))
			.expect("remove_panel: panel must be docked");
		let GridNode::Leaf(g) = self.grid.as_mut().unwrap().at_mut(&loc).unwrap() else {
			unreachable!()
		};
		if g.remove_tab(&panel) {
			dnd::prune_leaf(self, &loc);
			if self.active_group == Some(gid) {
				self.active_group = self.grid.as_ref().and_then(|grid| grid.leaves().first().map(|(_, g)| g.id));
			}
		}
		if let Some(grid) = self.grid.as_mut() {
			gridview::normalize(grid);
		}
		self.maximized = None; // a remove reshapes the tree, staling any maximized location.
		self.panels.remove(&panel);
	}

	pub fn maximize(&mut self, group: GroupId) {
		self.maximized = self.grid.as_ref().and_then(|grid| grid.locate(group));
	}

	pub fn exit_maximized(&mut self) {
		self.maximized = None;
	}

	pub fn float(&mut self, group: GroupId, rect: Rect) {
		let loc = self.grid.as_ref().expect("float: needs a grid").locate(group).expect("float: group must be docked");
		let GridNode::Leaf(g) = self.grid.as_ref().unwrap().at(&loc).unwrap() else {
			panic!("float: a group is always a leaf")
		};
		let detached = g.clone();
		dnd::prune_leaf(self, &loc);
		self.floating.push(FloatingGroup { group: detached, rect });
		if let Some(grid) = self.grid.as_mut() {
			gridview::normalize(grid);
		}
		self.maximized = None;
	}
}

/// Tree-independent metadata for a panel.
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct PanelMeta {
	pub title: String,
}
