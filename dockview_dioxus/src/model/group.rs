//! A tab-group: one grid leaf holding several panels with one active. Port of
//! `dockview-core/src/dockview/dockviewGroupPanel*` reduced to its data — the tab
//! list, the active index, and tab ops. The header/DOM live in [`crate::render`].

use crate::model::{GroupId, PanelId};

/// The model behind a single pane's tab strip (insilicoterminal's `.subtitlebar`).
#[derive(Clone, Debug, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct Group {
	pub id: GroupId,
	/// Tabs left→right; never empty (an emptied group is removed from the tree).
	pub tabs: Vec<PanelId>,
	/// Index into `tabs` of the visible panel.
	pub active: usize,
}

impl Group {
	pub fn new(id: GroupId, panel: PanelId) -> Self {
		Self { id, tabs: vec![panel], active: 0 }
	}

	pub fn active_panel(&self) -> &PanelId {
		&self.tabs[self.active]
	}

	/// Add a panel as a tab at `index` and activate it (center-drop / new tab).
	pub fn insert_tab(&mut self, _panel: PanelId, _index: usize) {
		todo!("insert tab, set active to it")
	}

	/// Remove `panel`; returns `true` if the group is now empty and should be
	/// pruned from the tree by the caller.
	pub fn remove_tab(&mut self, _panel: &PanelId) -> bool {
		todo!("remove tab, fix up active, report emptiness")
	}

	/// Reorder a tab within the strip (drag within the same header).
	pub fn move_tab(&mut self, _from: usize, _to: usize) {
		todo!("reorder tabs, keep active pointing at the same panel")
	}
}
