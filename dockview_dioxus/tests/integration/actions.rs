//! State-aware action enum + generator. Every action drives a *real* `pub` op on the live
//! [`PackedGrid`], and the generator enumerates only valid targets each step (matklad's
//! "non-crashed replicas only") so no entropy is spent on impossible picks.

use dockview_dioxus::{
	GroupId,
	model::packed::{DragSource, DropTarget},
};

use crate::{
	frng::Frng,
	sim::{COLS, World},
};

/// Action kinds, in the order their swarm weights are drawn.
pub const N_KINDS: usize = 5;
const PLACE: usize = 0;
const RESIZE: usize = 1;
const ADD_TAB: usize = 2;
const CLOSE: usize = 3;
const DROP: usize = 4;

#[derive(Clone, Debug)]
pub enum Action {
	Place { w: u32, h: u32, min_w: u32, min_h: u32 },
	Resize { idx: usize, new_w: u32, new_h: u32 },
	AddTab { group: GroupId },
	CloseActive { group: GroupId },
	Drop { source: DragSource, target: DropTarget },
}

pub fn apply(action: &Action, world: &mut World) {
	match action {
		Action::Place { w, h, min_w, min_h } => {
			let gid = world.grid.mint_group_id();
			let panel = world.mint_panel();
			world.grid.place(dockview_dioxus::Group::new(gid, panel), *w, *h, (*min_w, *min_h), COLS);
		}
		Action::Resize { idx, new_w, new_h } => world.grid.resize(*idx, *new_w, *new_h, COLS),
		Action::AddTab { group } => {
			let panel = world.mint_panel();
			world.grid.add_tab(*group, panel);
		}
		Action::CloseActive { group } => world.grid.close_active(*group),
		Action::Drop { source, target } => world.grid.drop(source.clone(), target.clone(), COLS),
	}
}

/// Pick a valid action for the current `world`, weighting kinds by the per-run `weights`
/// (swarm). Returns `None` only when the grid is empty and nothing but `Place` is possible but
/// weighted out — the run then ends.
pub fn generate(frng: &mut Frng, world: &World, weights: &[u32; N_KINDS]) -> Option<Action> {
	let cells = &world.grid.cells;

	let mut avail = vec![PLACE]; // placing a fresh tile is always possible.
	if !cells.is_empty() {
		avail.push(RESIZE);
		avail.push(ADD_TAB);
		avail.push(CLOSE);
		avail.push(DROP);
	}

	let ws: Vec<u32> = avail.iter().map(|&k| weights[k].max(1)).collect();
	let kind = avail[frng.weighted(&ws)];

	Some(match kind {
		PLACE => {
			let w = 1 + frng.below(4); // 1..=4, ≤ COLS
			let h = 1 + frng.below(4);
			Action::Place {
				w,
				h,
				min_w: 1 + frng.below(w),
				min_h: 1 + frng.below(h),
			}
		}
		RESIZE => {
			let idx = frng.below(cells.len() as u32) as usize;
			Action::Resize {
				idx,
				new_w: 1 + frng.below(5),
				new_h: 1 + frng.below(5),
			}
		}
		ADD_TAB => Action::AddTab { group: pick_group(frng, world) },
		CLOSE => Action::CloseActive { group: pick_group(frng, world) },
		DROP => {
			let source = {
				let c = &cells[frng.below(cells.len() as u32) as usize];
				// half the time tear a tab, else take the whole tile.
				if frng.below(2) == 0 {
					DragSource::Tab {
						panel: c.group.tabs[frng.below(c.group.tabs.len() as u32) as usize].clone(),
						from: c.group.id,
					}
				} else {
					DragSource::Tile(c.group.id)
				}
			};
			let target = match frng.below(3) {
				0 => DropTarget::Tab(cells[frng.below(cells.len() as u32) as usize].group.id),
				1 => {
					let c = &cells[frng.below(cells.len() as u32) as usize];
					DropTarget::Displace { x: c.x, y: c.y }
				}
				_ => DropTarget::Pack { x: frng.below(COLS) },
			};
			Action::Drop { source, target }
		}
		_ => unreachable!("kind is one of the constants pushed into `avail`"),
	})
}

fn pick_group(frng: &mut Frng, world: &World) -> GroupId {
	world.grid.cells[frng.below(world.grid.cells.len() as u32) as usize].group.id
}
