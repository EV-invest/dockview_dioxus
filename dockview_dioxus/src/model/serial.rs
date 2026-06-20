//! JSON (de)serialization of the layout. Port of the `serialize`/`deserialize`
//! halves of `dockview-core/src/gridview/gridview.ts` and `dockview/deserializer.ts`.
//!
//! [`DockModel`] already derives serde, so a layout round-trips to JSON directly —
//! `DockModel` *is* the serialized value, like react-mosaic's tree. This module is
//! the seam for keeping that JSON **stable across versions** (schema tag + migrations);
//! dockview's `validate.ts`/`fromJSON` resets on a corrupt layout rather than fall back.

use crate::model::DockModel;

/// Bump when the on-disk shape changes; `load` migrates older payloads.
pub const SCHEMA_VERSION: u32 = 1;

/// Versioned wrapper actually written to storage.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Serialized {
	pub version: u32,
	pub model: DockModel,
}

pub fn save(model: &DockModel) -> String {
	serde_json::to_string(&Serialized { version: SCHEMA_VERSION, model: model.clone() }).expect("DockModel is always serializable")
}

/// Parse a saved layout. Errors (not a silent default) on malformed/younger JSON so
/// a corrupt layout surfaces instead of silently wiping the user's workspace.
pub fn load(json: &str) -> Result<DockModel, LoadError> {
	let serialized: Serialized = serde_json::from_str(json).map_err(LoadError::Parse)?;
	if serialized.version > SCHEMA_VERSION {
		return Err(LoadError::FutureVersion(serialized.version));
	}
	Ok(serialized.model)
}

#[derive(Debug)]
pub enum LoadError {
	Parse(serde_json::Error),
	/// Payload is newer than this build understands.
	FutureVersion(u32),
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use super::*;
	use crate::model::{GroupId, PanelId, gridview::GridNode, group::Group};

	fn sample() -> DockModel {
		let mut m = DockModel { grid: Some(GridNode::Leaf(Group::new(GroupId(0), PanelId("a".into())))), floating: vec![], maximized: None, active_group: None, next_group_id: 0, panels: HashMap::new() };
		// drive next_group_id off its default so the round-trip actually proves it persists.
		let _ = m.mint_group_id();
		let _ = m.mint_group_id();
		m
	}

	#[test]
	fn round_trips_including_next_group_id() {
		let m = sample();
		let back = load(&save(&m)).expect("loads");
		assert_eq!(back.next_group_id, m.next_group_id, "next_group_id must survive to avoid id collisions");
		assert_eq!(back.grid, m.grid);
	}

	#[test]
	fn future_version_errors_not_resets() {
		let json = save(&sample()).replace("\"version\":1", &format!("\"version\":{}", SCHEMA_VERSION + 1));
		assert!(matches!(load(&json), Err(LoadError::FutureVersion(v)) if v == SCHEMA_VERSION + 1));
	}

	#[test]
	fn garbage_errors() {
		assert!(matches!(load("not json"), Err(LoadError::Parse(_))));
	}
}
