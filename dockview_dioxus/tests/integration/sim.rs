//! The run loop: build the world from `(seed, size)`, then generate → apply → assert the
//! full oracle after every step. A run is fully reproducible from its `(seed, size)`, so
//! `minimize` can re-run it freely.

use dockview_dioxus::{
	geometry::Position,
	model::{DockModel, DragSource, GridNode, Group, PanelId, PanelMeta, dnd::apply_drop},
};

use crate::{actions, frng::Frng, oracle};

/// An oracle violation. The reproducing trace is the `(seed, size)` itself — `replay`
/// re-runs verbosely to print the steps — so we carry only what the report prints.
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

	let mut model = seed_world();
	let mut initial_panels = oracle::live_panels(&model);
	initial_panels.sort();

	let mut step = 0;
	while frng.remaining() > 0 {
		let Some(action) = actions::generate(&mut frng, &model, &weights) else { break };
		// Print *before* applying so a production panic points at the culprit action.
		if verbose {
			eprintln!("step {step}: {action:?}");
		}
		actions::apply(&action, &mut model);
		if let Err(what) = oracle::check(&model, &initial_panels) {
			return Err(Failure { step, what });
		}
		step += 1;
	}
	Ok(())
}

/// The `basic.rs` 5-panel / 4-group starting layout, built through the real `apply_drop`
/// path so the fuzzer starts from realistic non-trivial state.
fn seed_world() -> DockModel {
	let names = ["watchlist", "notes", "chart", "orders", "console"];
	let mut m = DockModel::default();
	let gid = m.mint_group_id();
	let mut g = Group::new(gid, PanelId(names[0].to_string()));
	for n in &names[1..] {
		let idx = g.tabs.len();
		g.insert_tab(PanelId((*n).to_string()), idx);
	}
	g.active = 0;
	m.grid = Some(GridNode::Leaf(g));
	m.active_group = Some(gid);
	for n in &names {
		m.panels.insert(PanelId((*n).to_string()), PanelMeta { title: (*n).to_string() });
	}

	move_panel(&mut m, "chart", &[], Position::Right);
	move_panel(&mut m, "console", &[1], Position::Bottom);
	move_panel(&mut m, "orders", &[0], Position::Bottom);
	m
}

/// Mirror of `DockApi::move_panel`, inlined so the seed needs no Dioxus `Signal`.
fn move_panel(m: &mut DockModel, panel: &str, target: &[usize], position: Position) {
	let panel = PanelId(panel.to_string());
	let from_group = m
		.grid
		.as_ref()
		.expect("seed: grid")
		.leaves()
		.into_iter()
		.find(|(_, g)| g.tabs.contains(&panel))
		.map(|(_, g)| g.id)
		.expect("seed: panel is docked");
	apply_drop(m, DragSource::Tab { panel, from_group }, &target.to_vec(), position);
}
