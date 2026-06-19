//! Imperative handle over the model `Signal`. Port of `dockview-core/src/api/`
//! (`DockviewApi`): the small, stable surface a consumer scripts the layout with,
//! plus what the default layout/save/load flows call. Every method is a pure
//! mutation of [`DockModel`] behind a `Signal` write — no hidden DOM.

use dioxus::prelude::*;

use crate::{
	geometry::Position,
	model::{DockModel, GroupId, Location, PanelId},
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
	pub fn add_panel(&mut self, _panel: PanelId, _title: String, _target: Option<(Location, Position)>) {
		todo!("mint group as needed, mutate grid via model::dnd/gridview, normalize")
	}

	pub fn move_panel(&mut self, _panel: PanelId, _target: Location, _position: Position) {
		todo!("delegate to model::dnd::apply_drop")
	}

	pub fn remove_panel(&mut self, _panel: PanelId) {
		todo!("remove tab; prune empty group; normalize")
	}

	pub fn maximize(&mut self, _group: GroupId) {
		todo!("set model.maximized to the group's location")
	}

	pub fn exit_maximized(&mut self) {
		todo!("clear model.maximized")
	}

	pub fn float(&mut self, _group: GroupId, _rect: crate::math::Rect) {
		todo!("detach group from grid into model.floating")
	}

	/// Serialize the current layout (see [`crate::model::serial`]).
	pub fn save(&self) -> String {
		todo!("model::serial::save(&self.model.read())")
	}

	/// Replace the layout from a saved payload.
	pub fn load(&mut self, _json: &str) {
		todo!("model::serial::load -> set signal")
	}
}
