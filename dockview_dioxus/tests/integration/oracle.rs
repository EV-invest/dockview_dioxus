//! The contract checked after *every* action — independent of the implementation.
//! Three layers: structural tree invariants (re-implemented here so the oracle never
//! leans on prod's `cfg(test)` checker), panel conservation against the seed set, and a
//! serde round-trip. Returns `Err(reason)` rather than panicking so the minimizer
//! survives a violation (production `expect`/`unreachable` blowups are caught separately).

use std::collections::HashSet;

use dockview_dioxus::model::{DockModel, GridNode, GroupId, serial};

pub fn check(model: &DockModel, initial_panels: &[String]) -> Result<(), String> {
	if let Some(grid) = &model.grid {
		invariants(grid)?;
	}
	for fg in &model.floating {
		// A floating group must satisfy the same leaf invariants (non-empty, active in range).
		invariants(&GridNode::Leaf(fg.group.clone()))?;
	}

	let mut live = live_panels(model);
	live.sort();
	if live != initial_panels {
		return Err(format!("panel set drifted: live {live:?} != seed {initial_panels:?}"));
	}

	let mut seen = HashSet::new();
	for id in live_group_ids(model) {
		if !seen.insert(id.0) {
			return Err(format!("duplicate live group id {}", id.0));
		}
	}

	// Exact structural round-trip (ids, tabs, active, orientation, next_group_id, panels),
	// but tolerant of sub-ULP size drift: serde_json's f64 round-trip is not bit-exact
	// (a 50/50 split resized to 25.2941…6 reloads as …2), which is invisible under CSS
	// flex %. Round both sides to 1e-6 — far finer than any layout cares — then compare.
	let mut back = serial::load(&serial::save(model)).map_err(|e| format!("round-trip load failed: {e:?}"))?;
	let mut want = model.clone();
	if let Some(g) = back.grid.as_mut() {
		round_sizes(g);
	}
	if let Some(g) = want.grid.as_mut() {
		round_sizes(g);
	}
	if back != want {
		return Err("round-trip changed the model beyond float precision".to_string());
	}
	Ok(())
}

pub fn live_panels(model: &DockModel) -> Vec<String> {
	let mut out = Vec::new();
	if let Some(grid) = &model.grid {
		for (_, g) in grid.leaves() {
			out.extend(g.tabs.iter().map(|p| p.0.clone()));
		}
	}
	for fg in &model.floating {
		out.extend(fg.group.tabs.iter().map(|p| p.0.clone()));
	}
	out
}
fn round_sizes(node: &mut GridNode) {
	if let GridNode::Branch { children, .. } = node {
		for c in children.iter_mut() {
			c.size = (c.size * 1e6).round() / 1e6;
			round_sizes(&mut c.node);
		}
	}
}

fn invariants(node: &GridNode) -> Result<(), String> {
	match node {
		GridNode::Leaf(g) => {
			if g.tabs.is_empty() {
				return Err("empty group".to_string());
			}
			if g.active >= g.tabs.len() {
				return Err(format!("active index {} out of range (len {})", g.active, g.tabs.len()));
			}
		}
		GridNode::Branch { orientation, children } => {
			if children.len() < 2 {
				return Err(format!("branch has {} children (need ≥2)", children.len()));
			}
			let sum: f64 = children.iter().map(|c| c.size).sum();
			if (sum - 100.0).abs() >= 1e-6 {
				return Err(format!("branch sizes sum to {sum}, not 100"));
			}
			for c in children {
				if let GridNode::Branch { orientation: o, .. } = &c.node {
					if o == orientation {
						return Err("same-orientation parent/child nesting".to_string());
					}
				}
				invariants(&c.node)?;
			}
		}
	}
	Ok(())
}

fn live_group_ids(model: &DockModel) -> Vec<GroupId> {
	let mut out = Vec::new();
	if let Some(grid) = &model.grid {
		out.extend(grid.leaves().into_iter().map(|(_, g)| g.id));
	}
	out.extend(model.floating.iter().map(|fg| fg.group.id));
	out
}
