//! Pure scalar/geometry primitives. Port of `dockview-core/src/math.ts` + the
//! `Box`/`Rect` shapes used by the gridview and overlay layers.

/// Pixel rectangle in container-local coordinates — what the content overlay
/// layer positions each panel by (dockview's `getDomNodePagePosition` result).
#[derive(Clone, Copy, Debug, Default, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct Rect {
	pub x: f64,
	pub y: f64,
	pub width: f64,
	pub height: f64,
}

/// A node's measured extent, orientation-agnostic (dockview `GridNode.box`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Boxed {
	pub width: f64,
	pub height: f64,
}

pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
	debug_assert!(min <= max, "clamp: min must be <= max");
	value.max(min).min(max)
}
