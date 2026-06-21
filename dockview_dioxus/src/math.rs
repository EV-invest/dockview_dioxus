//! Pure geometry primitives.

/// A pixel rectangle (dockview's `getDomNodePagePosition` result). Measured slot/root
/// boxes are stored raw (viewport); the content overlay localizes them at render time.
#[derive(Clone, Copy, Debug, Default, serde::Deserialize, PartialEq, serde::Serialize)]
pub struct Rect {
	pub x: f64,
	pub y: f64,
	pub width: f64,
	pub height: f64,
}
