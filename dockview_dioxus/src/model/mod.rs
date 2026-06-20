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

use gridview::GridNode;
use group::Group;

/// Path of child indices from the grid root to a node (dockview `location: number[]`).
pub type Location = Vec<usize>;
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
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct FloatingGroup {
	pub group: Group,
	pub rect: crate::math::Rect,
}

/// The complete layout state. One of these lives in a `Signal`; everything renders
/// from it and every interaction is a pure mutation of it.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
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
}

/// Tree-independent metadata for a panel.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct PanelMeta {
	pub title: String,
}
