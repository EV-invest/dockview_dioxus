//! Imperative handle over the model `Signal`. Port of `dockview-core/src/api/`
//! (`DockviewApi`): the small, stable surface a consumer scripts the layout with,
//! plus what the default layout/save/load flows call. Every method is a pure
//! mutation of [`DockModel`] behind a `Signal` write — no hidden DOM.

use dioxus::prelude::*;

use crate::{
	geometry::Position,
	math::Rect,
	model::{DockModel, GroupId, Location, PanelId},
};

/// Cheap, `Copy` handle (wraps the `Signal`) shared via context so panels and
/// headers can drive the layout. Mirrors how `DockviewApi` is threaded through props.
///
/// Every layout op is a thin facade over the matching pure [`DockModel`] method — the
/// logic lives on the model (so it stays testable and fuzz-reachable); this just owns the
/// `Signal` write.
#[derive(Clone, Copy)]
pub struct DockApi {
	pub(crate) model: Signal<DockModel>,
}

impl DockApi {
	pub fn add_panel(&mut self, panel: PanelId, title: String, target: Option<(Location, Position)>) {
		self.model.write().add_panel(panel, title, target);
	}

	pub fn move_panel(&mut self, panel: PanelId, target: Location, position: Position) {
		self.model.write().move_panel(panel, target, position);
	}

	pub fn remove_panel(&mut self, panel: PanelId) {
		self.model.write().remove_panel(panel);
	}

	pub fn maximize(&mut self, group: GroupId) {
		self.model.write().maximize(group);
	}

	pub fn exit_maximized(&mut self) {
		self.model.write().exit_maximized();
	}

	pub fn float(&mut self, group: GroupId, rect: Rect) {
		self.model.write().float(group, rect);
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
