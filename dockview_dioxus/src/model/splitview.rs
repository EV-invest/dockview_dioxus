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
pub fn resize_pair(sizes: &mut [f64], index: usize, delta_pct: f64) {
	assert!(index + 1 < sizes.len(), "resize_pair: no sibling after index");
	let total = sizes[index] + sizes[index + 1];
	// REVIEW (fuzzer-found): a pair summing to < 2·MIN can't honour the floor on both sides,
	// so clamp the floor itself to total/2 — degenerate pairs split evenly instead of panicking
	// on an inverted clamp range. Reachable when `normalize` shrinks many siblings below MIN.
	let floor = MIN_CHILD_PCT.min(total / 2.0);
	let new_first = crate::math::clamp(sizes[index] + delta_pct, floor, total - floor);
	sizes[index] = new_first;
	sizes[index + 1] = total - new_first;
}

/// Insert a child at `index` sized per `sizing`, rescaling siblings to keep sum=100.
pub fn insert_child_size(sizes: &mut Vec<f64>, index: usize, sizing: Sizing) {
	let new_size = match sizing {
		Sizing::Distribute => 100.0 / (sizes.len() + 1) as f64,
		Sizing::Split(i) => {
			let half = sizes[i] / 2.0;
			sizes[i] = half;
			half
		}
	};
	sizes.insert(index, new_size);
	renormalize(sizes);
}

/// Remove the child at `index`, redistributing its share to the remaining siblings.
pub fn remove_child_size(sizes: &mut Vec<f64>, index: usize) {
	sizes.remove(index);
	renormalize(sizes);
}

/// Scale `sizes` so they sum to 100, preserving ratios. Proportional redistribution
/// falls out for free: every remaining child keeps its share of the new whole.
fn renormalize(sizes: &mut [f64]) {
	let sum: f64 = sizes.iter().sum();
	assert!(sum > 0.0, "renormalize: sizes must have positive sum");
	for s in sizes.iter_mut() {
		*s = *s / sum * 100.0;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn sum(s: &[f64]) -> f64 {
		s.iter().sum()
	}

	#[test]
	fn resize_moves_and_preserves_sum() {
		let mut s = vec![50.0, 50.0];
		resize_pair(&mut s, 0, 20.0);
		assert_eq!(s, vec![70.0, 30.0]);
		assert!((sum(&s) - 100.0).abs() < 1e-9);
	}

	#[test]
	fn resize_clamps_to_min() {
		let mut s = vec![50.0, 50.0];
		resize_pair(&mut s, 0, 100.0); // would zero the sibling
		assert_eq!(s[1], MIN_CHILD_PCT);
		assert!((sum(&s) - 100.0).abs() < 1e-9);
	}

	#[test]
	fn insert_distribute_keeps_sum() {
		let mut s = vec![50.0, 50.0];
		insert_child_size(&mut s, 1, Sizing::Distribute);
		assert_eq!(s.len(), 3);
		assert!((sum(&s) - 100.0).abs() < 1e-9);
	}

	#[test]
	fn insert_split_halves_target() {
		let mut s = vec![60.0, 40.0];
		insert_child_size(&mut s, 1, Sizing::Split(0));
		assert!((sum(&s) - 100.0).abs() < 1e-9);
		assert!((s[0] - s[1]).abs() < 1e-9); // 60 halved → two equal 30s, then renormalized
	}

	#[test]
	fn remove_redistributes_proportionally() {
		let mut s = vec![20.0, 30.0, 50.0];
		remove_child_size(&mut s, 0);
		assert!((sum(&s) - 100.0).abs() < 1e-9);
		// 30:50 ratio preserved → 37.5:62.5
		assert!((s[0] - 37.5).abs() < 1e-9);
		assert!((s[1] - 62.5).abs() < 1e-9);
	}
}
