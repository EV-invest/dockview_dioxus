//! Orientation + drop-zone geometry. Pure functions ported from
//! `dockview-core/src/dnd/droptarget.ts` (`calculateQuadrant*`) and the
//! location helpers in `gridview.ts` (`getRelativeLocation`, `getDirectionOrientation`).
//!
//! These are the parts of dockview that are *already* pure logic — they port
//! almost verbatim and need no DOM.

/// Split axis of a branch. `Horizontal` lays children left→right, `Vertical` top→bottom.
/// (dockview `Orientation`.)
#[derive(Clone, Copy, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub enum Orientation {
	Horizontal,
	Vertical,
}

impl Orientation {
	/// The perpendicular axis — gridview alternates orientation at each depth.
	pub fn orthogonal(self) -> Self {
		match self {
			Orientation::Horizontal => Orientation::Vertical,
			Orientation::Vertical => Orientation::Horizontal,
		}
	}
}

/// One of the five drop zones a panel exposes while a drag hovers it.
/// `Center` docks as a tab; the edges split the target into a new branch.
/// (dockview `Position`.)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Position {
	Top,
	Bottom,
	Left,
	Right,
	Center,
}

impl Position {
	/// The split axis an edge-drop produces (dockview `getDirectionOrientation`).
	/// `Center` never splits, so returns `None`.
	pub fn split_orientation(self) -> Option<Orientation> {
		match self {
			Position::Top | Position::Bottom => Some(Orientation::Vertical),
			Position::Left | Position::Right => Some(Orientation::Horizontal),
			Position::Center => None,
		}
	}
}

/// Resolve which of the five zones a pointer at (`x`,`y`) within a `width`×`height`
/// pane falls into, given which zones the target accepts. `threshold` is the edge
/// band as a percentage (dockview default 20). Verbatim port of
/// `calculateQuadrantAsPercentage`.
pub fn quadrant_at(accepted: &[Position], x: f64, y: f64, width: f64, height: f64, threshold: f64) -> Option<Position> {
	let xp = 100.0 * x / width;
	let yp = 100.0 * y / height;

	if accepted.contains(&Position::Left) && xp < threshold {
		return Some(Position::Left);
	}
	if accepted.contains(&Position::Right) && xp > 100.0 - threshold {
		return Some(Position::Right);
	}
	if accepted.contains(&Position::Top) && yp < threshold {
		return Some(Position::Top);
	}
	if accepted.contains(&Position::Bottom) && yp > 100.0 - threshold {
		return Some(Position::Bottom);
	}
	accepted.contains(&Position::Center).then_some(Position::Center)
}

#[cfg(test)]
mod tests {
	use super::*;

	const ALL: &[Position] = &[Position::Top, Position::Bottom, Position::Left, Position::Right, Position::Center];

	#[test]
	fn dead_centre_is_center() {
		assert_eq!(quadrant_at(ALL, 50.0, 50.0, 100.0, 100.0, 20.0), Some(Position::Center));
	}

	#[test]
	fn edges_resolve_within_band() {
		assert_eq!(quadrant_at(ALL, 10.0, 50.0, 100.0, 100.0, 20.0), Some(Position::Left));
		assert_eq!(quadrant_at(ALL, 90.0, 50.0, 100.0, 100.0, 20.0), Some(Position::Right));
		assert_eq!(quadrant_at(ALL, 50.0, 10.0, 100.0, 100.0, 20.0), Some(Position::Top));
		assert_eq!(quadrant_at(ALL, 50.0, 90.0, 100.0, 100.0, 20.0), Some(Position::Bottom));
	}

	#[test]
	fn just_inside_the_band_falls_to_center() {
		// xp = 21 > threshold 20 → no left, no other edge → center
		assert_eq!(quadrant_at(ALL, 21.0, 50.0, 100.0, 100.0, 20.0), Some(Position::Center));
	}

	#[test]
	fn unaccepted_center_yields_none() {
		let edges = &[Position::Left, Position::Right];
		assert_eq!(quadrant_at(edges, 50.0, 50.0, 100.0, 100.0, 20.0), None);
	}

	#[test]
	fn corner_prefers_horizontal_edge() {
		// left checked before top, matching dockview's order
		assert_eq!(quadrant_at(ALL, 5.0, 5.0, 100.0, 100.0, 20.0), Some(Position::Left));
	}
}
