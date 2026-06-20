//! Imperative handle over the model `Signal`. Port of `dockview-core/src/api/`
//! (`DockviewApi`): the small, stable surface a consumer scripts the layout with,
//! plus what the default layout/save/load flows call. Every method is a pure
//! mutation of [`DockModel`] behind a `Signal` write — no hidden DOM.

use dioxus::prelude::*;

use crate::{
	geometry::Position,
	math::Rect,
	model::{
		DockModel, FloatingGroup, GroupAddr, GroupId, Location, PanelId, PanelMeta,
		dnd::{DragSource, apply_drop, prune_leaf},
		gridview::{GridNode, insert_split, normalize},
		group::Group,
	},
};

/// Cheap, `Copy` handle (wraps the `Signal`) shared via context so panels and
/// headers can drive the layout. Mirrors how `DockviewApi` is threaded through props.
#[derive(Clone, Copy)]
pub struct DockApi {
	pub(crate) model: Signal<DockModel>,
}

impl DockApi {
	/// Dock a panel relative to an existing group (or as the first panel when the
	/// grid is empty). Core entry point — `DockviewApi.addPanel`.
	pub fn add_panel(&mut self, panel: PanelId, title: String, target: Option<(Location, Position)>) {
		let mut m = self.model.write();
		m.panels.insert(panel.clone(), PanelMeta { title });

		if m.grid.is_none() {
			let gid = m.mint_group_id();
			m.grid = Some(GridNode::Leaf(Group::new(gid, panel)));
			m.active_group = Some(gid);
			return;
		}

		match target {
			Some((loc, Position::Center)) => {
				let g = m.group_mut(&GroupAddr::Docked(loc));
				let idx = g.tabs.len();
				g.insert_tab(panel, idx);
			}
			Some((loc, edge)) => {
				let gid = m.mint_group_id();
				insert_split(m.grid.as_mut().unwrap(), &loc, edge, Group::new(gid, panel));
				m.maximized = None; // the split shifts locations, staling any maximized one.
			}
			None => {
				// Tab into the active group, or the first leaf if there is none.
				let loc = m
					.active_group
					.and_then(|gid| m.grid.as_ref().unwrap().locate(gid))
					.or_else(|| m.grid.as_ref().unwrap().leaves().first().map(|(l, _)| l.clone()))
					.expect("add_panel: a non-empty grid has at least one leaf");
				let g = m.group_mut(&GroupAddr::Docked(loc));
				let idx = g.tabs.len();
				g.insert_tab(panel, idx);
			}
		}
	}

	pub fn move_panel(&mut self, panel: PanelId, target: Location, position: Position) {
		let mut m = self.model.write();
		let from_group = m
			.grid
			.as_ref()
			.expect("move_panel: needs a grid")
			.leaves()
			.into_iter()
			.find(|(_, g)| g.tabs.contains(&panel))
			.map(|(_, g)| g.id)
			.expect("move_panel: panel must already be docked");
		apply_drop(&mut m, DragSource::Tab { panel, from_group }, &target, position);
	}

	pub fn remove_panel(&mut self, panel: PanelId) {
		let mut m = self.model.write();
		let (loc, gid) = m
			.grid
			.as_ref()
			.expect("remove_panel: needs a grid")
			.leaves()
			.into_iter()
			.find(|(_, g)| g.tabs.contains(&panel))
			.map(|(l, g)| (l, g.id))
			.expect("remove_panel: panel must be docked");
		let GridNode::Leaf(g) = m.grid.as_mut().unwrap().at_mut(&loc).unwrap() else { unreachable!() };
		if g.remove_tab(&panel) {
			prune_leaf(&mut m, &loc);
			if m.active_group == Some(gid) {
				m.active_group = m.grid.as_ref().and_then(|grid| grid.leaves().first().map(|(_, g)| g.id));
			}
		}
		if let Some(grid) = m.grid.as_mut() {
			normalize(grid);
		}
		m.maximized = None; // a remove reshapes the tree, staling any maximized location.
		m.panels.remove(&panel);
	}

	pub fn maximize(&mut self, group: GroupId) {
		let mut m = self.model.write();
		m.maximized = m.grid.as_ref().and_then(|grid| grid.locate(group));
	}

	pub fn exit_maximized(&mut self) {
		self.model.write().maximized = None;
	}

	pub fn float(&mut self, group: GroupId, rect: Rect) {
		let mut m = self.model.write();
		let loc = m.grid.as_ref().expect("float: needs a grid").locate(group).expect("float: group must be docked");
		let GridNode::Leaf(g) = m.grid.as_ref().unwrap().at(&loc).unwrap() else { panic!("float: a group is always a leaf") };
		let detached = g.clone();
		prune_leaf(&mut m, &loc);
		m.floating.push(FloatingGroup { group: detached, rect });
		if let Some(grid) = m.grid.as_mut() {
			normalize(grid);
		}
		m.maximized = None;
	}

	/// Serialize the current layout (see [`crate::model::serial`]).
	pub fn save(&self) -> String {
		crate::model::serial::save(&self.model.read())
	}

	/// Replace the layout from a saved payload. A script-driven load: a corrupt payload
	/// here is a caller bug, so we panic loudly (unlike the restore path, which watermarks).
	pub fn load(&mut self, json: &str) {
		let model = crate::model::serial::load(json).expect("DockApi::load: corrupt layout");
		self.model.set(model);
	}
}
