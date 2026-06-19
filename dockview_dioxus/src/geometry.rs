//! Orientation + drop-zone geometry. Pure functions ported from
//! `dockview-core/src/dnd/droptarget.ts` (`calculateQuadrant*`) and the
//! location helpers in `gridview.ts` (`getRelativeLocation`, `getDirectionOrientation`).
//!
//! These are the parts of dockview that are *already* pure logic — they port
//! almost verbatim and need no DOM.

use crate::math::clamp;

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
pub fn quadrant_at(_accepted: &[Position], _x: f64, _y: f64, _width: f64, _height: f64, _threshold: f64) -> Option<Position> {
	let _ = clamp; // keep the dependency wired until the body lands
	todo!("port calculateQuadrantAsPercentage from droptarget.ts")
}
