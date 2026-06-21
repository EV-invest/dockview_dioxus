//! State-aware action enum + generator. Every action drives a *real* `pub` reshaper on
//! the live `DockModel`, and the generator enumerates only valid targets each step
//! (matklad's "non-crashed replicas only") so no entropy is spent on impossible picks.

use dockview_dioxus::{
	GroupId,
	geometry::Position,
	math::Rect,
	model::{DockModel, DragSource, GridNode, Group, Location, dnd::apply_drop, gridview::resize_branch},
};

use crate::frng::Frng;

const POSITIONS: [Position; 5] = [Position::Top, Position::Bottom, Position::Left, Position::Right, Position::Center];

/// Action kinds, in the order their swarm weights are drawn.
pub const N_KINDS: usize = 5;
const DROP: usize = 0;
const RESIZE: usize = 1;
const ACTIVATE: usize = 2;
const MOVE_TAB: usize = 3;
const FLOAT: usize = 4;

#[derive(Clone, Debug)]
pub enum Action {
	Drop {
		source: DragSource,
		target: Location,
		position: Position,
	},
	Resize {
		parent: Location,
		index: usize,
		delta_pct: f64,
	},
	Activate {
		loc: Location,
		index: usize,
	},
	MoveTab {
		loc: Location,
		from: usize,
		to: usize,
	},
	/// Detach a docked group to a floating overlay (re-dockable via a later `Drop`).
	Float {
		group: GroupId,
	},
}

pub fn apply(action: &Action, model: &mut DockModel) {
	match action {
		Action::Drop { source, target, position } => apply_drop(model, source.clone(), target, *position),
		Action::Resize { parent, index, delta_pct } => {
			resize_branch(model.grid.as_mut().expect("resize: grid exists"), parent, *index, *delta_pct);
		}
		Action::Activate { loc, index } => {
			let GridNode::Leaf(g) = model.grid.as_mut().expect("activate: grid").at_mut(loc).expect("activate: loc resolves") else {
				panic!("activate: target must be a leaf");
			};
			g.active = *index;
		}
		Action::MoveTab { loc, from, to } => {
			let GridNode::Leaf(g) = model.grid.as_mut().expect("move_tab: grid").at_mut(loc).expect("move_tab: loc resolves") else {
				panic!("move_tab: target must be a leaf");
			};
			g.move_tab(*from, *to);
		}
		Action::Float { group } => model.float(
			*group,
			Rect {
				x: 50.0,
				y: 50.0,
				width: 200.0,
				height: 150.0,
			},
		),
	}
}

/// Pick a valid action for the current `model`, weighting kinds by the per-run `weights`
/// (swarm). Returns `None` when nothing is possible (the run then ends).
pub fn generate(frng: &mut Frng, model: &DockModel, weights: &[u32; N_KINDS]) -> Option<Action> {
	let grid = model.grid.as_ref()?;
	let leaves = grid.leaves();
	if leaves.is_empty() {
		return None;
	}

	let mut brs = Vec::new();
	collect_branches(grid, &mut Vec::new(), &mut brs);
	let multi_tab: Vec<(Location, &Group)> = leaves.iter().filter(|(_, g)| g.tabs.len() >= 2).map(|(l, g)| (l.clone(), *g)).collect();
	// A drop needs a target leaf *and* a source from a different group: ≥2 leaves, or any float.
	let drop_ok = leaves.len() >= 2 || !model.floating.is_empty();

	let mut avail: Vec<usize> = Vec::new();
	if drop_ok {
		avail.push(DROP);
	}
	if !brs.is_empty() {
		avail.push(RESIZE);
	}
	if !multi_tab.is_empty() {
		avail.push(ACTIVATE);
		avail.push(MOVE_TAB);
	}
	// Float detaches a docked leaf; always possible while the grid holds one.
	avail.push(FLOAT);
	if avail.is_empty() {
		return None;
	}

	let ws: Vec<u32> = avail.iter().map(|&k| weights[k].max(1)).collect();
	let kind = avail[frng.weighted(&ws)];

	match kind {
		DROP => {
			let (target, tgt_group) = {
				let (loc, g) = &leaves[frng.below(leaves.len() as u32) as usize];
				(loc.clone(), g.id)
			};
			// Sources from any group *other than the target's* (the UI never drops a group on
			// itself — that would detach the very leaf we re-home into).
			let mut sources: Vec<DragSource> = Vec::new();
			for (_, g) in &leaves {
				if g.id == tgt_group {
					continue;
				}
				sources.push(DragSource::Group(g.id));
				for p in &g.tabs {
					sources.push(DragSource::Tab { panel: p.clone(), from_group: g.id });
				}
			}
			for fg in &model.floating {
				sources.push(DragSource::Group(fg.group.id));
				for p in &fg.group.tabs {
					sources.push(DragSource::Tab {
						panel: p.clone(),
						from_group: fg.group.id,
					});
				}
			}
			let source = sources[frng.below(sources.len() as u32) as usize].clone();
			let position = POSITIONS[frng.below(5) as usize];
			Some(Action::Drop { source, target, position })
		}
		RESIZE => {
			let (parent, count) = brs[frng.below(brs.len() as u32) as usize].clone();
			let index = frng.below((count - 1) as u32) as usize;
			let delta_pct = (frng.unit() * 2.0 - 1.0) * 60.0;
			Some(Action::Resize { parent, index, delta_pct })
		}
		ACTIVATE => {
			let (loc, g) = &multi_tab[frng.below(multi_tab.len() as u32) as usize];
			let index = frng.below(g.tabs.len() as u32) as usize;
			Some(Action::Activate { loc: loc.clone(), index })
		}
		MOVE_TAB => {
			let (loc, g) = &multi_tab[frng.below(multi_tab.len() as u32) as usize];
			let from = frng.below(g.tabs.len() as u32) as usize;
			let to = frng.below(g.tabs.len() as u32) as usize;
			Some(Action::MoveTab { loc: loc.clone(), from, to })
		}
		FLOAT => {
			let (_, g) = &leaves[frng.below(leaves.len() as u32) as usize];
			Some(Action::Float { group: g.id })
		}
		_ => unreachable!("kind is one of the constants pushed into `avail`"),
	}
}

/// Every branch location with its child count (for splitter-resize targeting).
fn collect_branches(node: &GridNode, path: &mut Location, out: &mut Vec<(Location, usize)>) {
	if let GridNode::Branch { children, .. } = node {
		out.push((path.clone(), children.len()));
		for (i, c) in children.iter().enumerate() {
			path.push(i);
			collect_branches(&c.node, path, out);
			path.pop();
		}
	}
}
