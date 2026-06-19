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

pub fn save(_model: &DockModel) -> String {
	todo!("wrap in Serialized with version=SCHEMA_VERSION, then serde_json::to_string")
}

/// Parse a saved layout. Errors (not a silent default) on malformed/younger JSON so
/// a corrupt layout surfaces instead of silently wiping the user's workspace.
pub fn load(_json: &str) -> Result<DockModel, LoadError> {
	todo!("deserialize, check version, migrate or error")
}

#[derive(Debug)]
pub enum LoadError {
	Parse(serde_json::Error),
	/// Payload is newer than this build understands.
	FutureVersion(u32),
}
