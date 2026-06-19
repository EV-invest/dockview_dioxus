//! 1-D proportional sizing. Port of `dockview-core/src/splitview/` — but trimmed:
//! dockview reimplements VSCode's full pixel layout engine (min/max/snap/priority,
//! `layout(size, orthogonalSize)`) because it predates reliable flexbox and must
//! drive popout windows frame-by-frame. We delegate actual layout to CSS flexbox
//! (`flex-basis: %` + `min-*`), so only the *resize math* survives here: how a
//! splitter drag redistributes percentages, and how a new child is sized.
//!
//! Sizes are percentages of the parent along its split axis and always sum to 100.

/// Minimum size any child may shrink to (percent). CSS `min-width/height` enforces
/// the pixel floor; this keeps the model honest so panels never serialize to ~0.
pub const MIN_CHILD_PCT: f64 = 5.0;

/// How a freshly inserted child claims its space (dockview `Sizing`).
#[derive(Clone, Copy, Debug)]
pub enum Sizing {
	/// Re-even all siblings (dockview `Distribute`).
	Distribute,
	/// Halve the sibling at `index`, taking half (dockview `Split`).
	Split(usize),
}

/// Drag the splitter between children `index` and `index+1` by `delta_pct`, moving
/// that percentage from one to the other, clamped so neither drops below
/// [`MIN_CHILD_PCT`]. Pure; the only sizing math a CSS-laid-out grid needs.
/// Port of `Splitview.resize`/`onSashDrag` reduced to the proportional case.
pub fn resize_pair(_sizes: &mut [f64], _index: usize, _delta_pct: f64) {
	todo!("move delta between adjacent siblings, clamp to MIN_CHILD_PCT, preserve sum=100")
}

/// Insert a child at `index` sized per `sizing`, rescaling siblings to keep sum=100.
pub fn insert_child_size(_sizes: &mut Vec<f64>, _index: usize, _sizing: Sizing) {
	todo!("grow room for a new child per Sizing, renormalize to 100")
}

/// Remove the child at `index`, redistributing its share to the remaining siblings.
pub fn remove_child_size(_sizes: &mut Vec<f64>, _index: usize) {
	todo!("drop a child, redistribute its % proportionally, keep sum=100")
}
