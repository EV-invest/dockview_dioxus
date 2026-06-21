//! The run loop: build the world from `(seed, size)`, then generate → apply → assert the full
//! oracle after every step. A run is fully reproducible from its `(seed, size)`, so `minimize`
//! can re-run it freely.

use dockview_dioxus::{
	GroupId, PanelId,
	model::{Group, packed::PackedGrid},
};

use crate::{actions, frng::Frng, oracle};

/// Fixed view width for the fuzzer — wide enough to admit side-by-side packing.
pub const COLS: u32 = 12;

/// The fuzzed world: the grid under test plus a monotonic panel-id source (the grid mints its
/// own group ids). Both action generation and application read/write through this.
pub struct World {
	pub grid: PackedGrid,
	pub next_panel: u64,
}

impl World {
	pub fn mint_panel(&mut self) -> PanelId {
		let id = PanelId(format!("p{}", self.next_panel));
		self.next_panel += 1;
		id
	}
}

/// An oracle violation. The reproducing trace is the `(seed, size)` itself — `replay` re-runs
/// verbosely to print the steps — so we carry only what the report prints.
pub struct Failure {
	pub step: usize,
	pub what: String,
}

pub fn run(seed: u64, size: usize, verbose: bool) -> Result<(), Failure> {
	let mut frng = Frng::new(seed, size);
	// Swarm: per-run action weights come from the buffer head, not hand-tuned constants.
	let mut weights = [0u32; actions::N_KINDS];
	for w in &mut weights {
		*w = frng.byte() as u32 % 8;
	}

	let mut world = seed_world();

	let mut step = 0;
	while frng.remaining() > 0 {
		let Some(action) = actions::generate(&mut frng, &world, &weights) else { break };
		if verbose {
			eprintln!("step {step}: {action:?}");
		}
		actions::apply(&action, &mut world);
		if let Err(what) = oracle::check(&world.grid, COLS) {
			return Err(Failure { step, what });
		}
		step += 1;
	}
	Ok(())
}

/// A small starting layout (a few packed tiles, one with a second tab), built through the real
/// `place`/`add_tab` path so the fuzzer starts from realistic non-trivial state.
fn seed_world() -> World {
	let mut world = World {
		grid: PackedGrid::default(),
		next_panel: 0,
	};
	for (w, h) in [(3, 4), (4, 3), (3, 5), (5, 4)] {
		let gid = world.grid.mint_group_id();
		let panel = world.mint_panel();
		world.grid.place(Group::new(gid, panel), w, h, (1, 1), COLS);
	}
	// give the first tile a second tab so tab-tears have something to bite on from the start.
	let extra = world.mint_panel();
	world.grid.add_tab(GroupId(0), extra);
	world
}
