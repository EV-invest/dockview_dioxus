//! Render layer for the packed-grid model ([`model::packed`](crate::model::packed)) —
//! fixed-size tiles that snap to a step grid and leave whitespace instead of filling
//! the view. InsilicoTerminal's look.
//!
//! The skeleton ([`PackedFrame`]) and the content overlay ([`PackedContent`]) are both
//! positioned directly from the model's integer grid rects (`x·STEP, y·STEP, …`), in the
//! packed root's own coordinate space. There is no DOM measuring: a tile and its content
//! share the exact same math, so content can never drift off its tile when the layout
//! settles (a measured slot only re-measures on *resize*, never on a pure move — the source
//! of the old "content nested in a neighbour" bug).
//!
//! Drag-to-reposition is a pure grid op: pick up a tile (titlebar) or tear a tab, and a
//! cloned [`PackedGrid::drop`] previews the result live (`view`) — other tiles settling under
//! gravity, the source's landing cell drawn as a detail-less grey shadow — committed verbatim on
//! release. The picked-up pane itself rides a floating ghost under the pointer (it does not snap);
//! only its shadow snaps ahead to where it will land. One `drop` path for preview and commit.

use std::{
	collections::{HashMap, HashSet},
	rc::Rc,
};

use dioxus::{html::input_data::MouseButton, prelude::*};

use super::CSS;
use crate::{
	config::Config,
	math::Rect,
	model::{
		Group, GroupId, PanelId,
		packed::{DragSource, DropTarget, GridState, MinSize, PackedGrid},
		serial,
	},
	panel::DockPanel,
};

/// Px per [`Step`](crate::model::packed::Step) — the min resize/snap increment.
const STEP: f64 = 120.0;
/// Approx root font size; resolves a type's rem-expressed [`MinSize`] to whole steps.
const REM_PX: f64 = 16.0;
/// Fixed header-bar height (CSS pins it); content starts below it.
const CHROME_H: f64 = 32.0;
/// A press promotes to a drag only past this many px, so a click still activates a tab.
const DRAG_THRESHOLD: f64 = 4.0;

/// Imperative handle over the packed grid: thin `Signal` writes over [`PackedGrid`],
/// with `place` resolving [`MinSize`] → steps first. `cols` is the live column count
/// the render root measures.
#[derive(Clone, Copy)]
pub struct PackedApi {
	pub grid: Signal<PackedGrid>,
	pub cols: Signal<u32>,
}

impl PackedApi {
	pub fn place(&mut self, group: Group, w: u32, h: u32, min: MinSize) {
		let cols = (self.cols)();
		self.grid.write().place(group, w, h, min.resolve(STEP, REM_PX), cols);
	}

	pub fn add_tab(&mut self, group: GroupId, panel: PanelId) {
		self.grid.write().add_tab(group, panel);
	}

	pub fn close_active(&mut self, group: GroupId) {
		self.grid.write().close_active(group);
	}

	pub fn resize(&mut self, idx: usize, new_w: u32, new_h: u32) {
		let cols = (self.cols)();
		self.grid.write().resize(idx, new_w, new_h, cols);
	}

	/// Serialize the current layout (see [`serial`](crate::model::serial)).
	pub fn save(&self) -> String {
		serial::save(&self.grid.read())
	}

	/// Replace the layout with a previously [`save`](Self::save)d one; errors (not a silent
	/// reset) on a corrupt or future-version payload.
	pub fn load(&mut self, json: &str) -> Result<(), serial::LoadError> {
		*self.grid.write() = serial::load(json)?;
		Ok(())
	}
}

/// Root of the packed layout. Owns the `Signal<PackedGrid>`, measures only its own width
/// (→ `cols = floor(width / STEP)`) and top-left origin (to map pointer→grid space), provides
/// [`PackedApi`]/the panels signal/the drag signal/the preview `view` via context, and stacks
/// the tile skeleton over the content overlay.
///
/// - `panels` is a `Signal` so windows spawned at runtime appear in the overlay.
/// - `on_ready`: invoked once with the [`PackedApi`] after the first measure (so seeds can
///   `place` against a real column count), letting a host script the initial tiles.
#[component]
pub fn PackedArea(panels: Signal<Vec<DockPanel>>, on_ready: Option<Callback<PackedApi>>, config: Option<Config>) -> Element {
	let cfg = config.unwrap_or_default();
	// Owned by the root, not this scope: `PackedApi` is handed to the host via `on_ready` and
	// driven from outside `PackedArea`'s subtree, so the signals must outlive this component.
	let mut grid = use_hook(|| Signal::new_in_scope(PackedGrid::default(), ScopeId::ROOT));
	let mut cols = use_hook(|| Signal::new_in_scope(0u32, ScopeId::ROOT));
	let api = PackedApi { grid, cols };
	let mut drag = use_signal(|| None::<Drag>);
	let mut root_origin = use_signal(|| (0.0_f64, 0.0_f64));
	// The pane keyboard ops act on: set when a tile's header/tab is pressed. `maximized` is a pure
	// view toggle (no model mutation) — the focused tile fills the container, the rest are hidden.
	let mut focused = use_signal(|| None::<GroupId>);
	let mut maximized = use_signal(|| None::<GroupId>);

	// Undo history of *solid* layouts: a snapshot is captured only when the grid is at rest
	// (no drag in flight, not mid-resize) and differs from the cursor's snapshot. Because
	// restoring sets `grid` back to exactly `states[cursor]`, an undo/redo write is a no-op
	// here — it won't re-record itself. A normal edit truncates any redo branch.
	let mut history = use_signal(History::default);
	use_effect(move || {
		if drag.read().is_some() {
			return;
		}
		let g = grid.read();
		// `cells.is_empty()` skips the default grid before the host's seed lands, so undo can't
		// walk back to a blank layout.
		if g.state != GridState::Settled || g.cells.is_empty() {
			return;
		}
		let mut h = history.write();
		if h.states.get(h.cursor) == Some(&*g) {
			return;
		}
		h.push(g.clone());
	});

	// The grid the skeleton/content actually render: the live preview while an armed drag is in
	// flight (other tiles settled under a cloned `drop`), else the real grid. One `drop` path.
	let view = use_memo(move || {
		let g = grid.read().clone();
		match drag.read().clone() {
			Some(d) if d.armed => match d.target {
				Some(t) => {
					let mut p = g;
					p.drop(d.source, t, cols());
					p
				}
				None => g,
			},
			_ => g,
		}
	});

	use_context_provider(|| api);
	use_context_provider(|| panels);
	use_context_provider(|| drag);
	use_context_provider(|| view);
	use_context_provider(|| PaneView { focused, maximized });
	let mut root_handle = use_signal(|| None::<Rc<MountedData>>);
	let mut ready = use_signal(|| false);

	// Seed once, but only after the first measure lands a real column count — placing into a
	// zero-column grid would degenerate every tile to x=0.
	use_effect(move || {
		if ready() || cols() == 0 {
			return;
		}
		if let Some(cb) = on_ready {
			cb.call(api);
		}
		ready.set(true);
	});

	let measure = move |h: Rc<MountedData>| async move {
		if let Ok(rect) = h.get_client_rect().await {
			let r: Rect = rect.into();
			cols.set((r.width / STEP).floor() as u32);
			root_origin.set((r.x, r.y));
		}
	};

	// The floating ghost of whatever is being dragged: it tracks the pointer 1:1 (`cursor − grab`),
	// keeping the grabbed point under the cursor, while its shadow snaps to the landing cell.
	let ghost = drag.read().clone().filter(|d| d.armed).map(|d| {
		let titles: HashMap<PanelId, String> = panels.read().iter().map(|p| (p.id.clone(), p.title.clone())).collect();
		let title = match &d.source {
			DragSource::Tile(g) => grid
				.read()
				.cells
				.iter()
				.find(|c| c.group.id == *g)
				.map(|c| titles.get(c.group.active_panel()).cloned().unwrap_or_default())
				.unwrap_or_default(),
			DragSource::Tab { panel, .. } => titles.get(panel).cloned().unwrap_or_default(),
		};
		(title, d.cursor.0 - d.grab.0, d.cursor.1 - d.grab.1, d.src_w as f64 * STEP, d.src_h as f64 * STEP)
	});

	let ids: Vec<u64> = view.read().cells.iter().map(|c| c.group.id.0).collect();
	rsx! {
		style { dangerous_inner_html: CSS }
		div {
			class: "dv-packed",
			tabindex: 0,
			onkeydown: move |e: KeyboardEvent| {
				let kb = cfg.keybinds;
				if kb.undo.matches(&e) {
					e.prevent_default();
					if let Some(g) = history.write().step(-1) {
						*grid.write() = g;
					}
				} else if kb.redo.matches(&e) {
					e.prevent_default();
					if let Some(g) = history.write().step(1) {
						*grid.write() = g;
					}
				} else if kb.close.matches(&e) {
					e.prevent_default();
					if let Some(g) = focused() {
						let mut api = api;
						api.close_active(g);
						// Dropped the last tab → the group is gone; don't leave focus/maximize on a dead id.
						if !grid.read().cells.iter().any(|c| c.group.id == g) {
							focused.set(None);
							if maximized() == Some(g) {
								maximized.set(None);
							}
						}
					}
				} else if kb.maximize.matches(&e) {
					e.prevent_default();
					let f = focused();
					maximized.set(if maximized() == f { None } else { f });
				}
			},
			onmounted: move |e| {
				let h = e.data();
				root_handle.set(Some(h.clone()));
				measure(h)
			},
			onresize: move |_| {
				let handle = root_handle();
				async move {
					if let Some(h) = handle {
						measure(h).await;
					}
				}
			},
			for (idx, id) in ids.iter().enumerate() {
				PackedFrame { key: "{id}", idx }
			}
			div { class: "dv-overlay", PackedContent {} }

			// Drag capture: a fixed surface (the Dioxus-web stand-in for `setPointerCapture`)
			// that owns pointermove/up for the whole gesture, so moves over child tiles don't
			// leak. Inlined (not a component) so it shares the root's signals without `PartialEq`
			// props. Arms past the threshold, resolves the live target each move, and on release
			// runs the *same* `drop` the preview used; an unarmed tab release is a plain click.
			if drag.read().is_some() {
				div {
					style: "position:fixed; inset:0; z-index:1000; cursor:grabbing;",
					onpointermove: move |e: PointerEvent| {
						let Some(mut d) = drag() else { return };
						let c = e.client_coordinates();
						d.cursor = (c.x, c.y);
						if !d.armed {
							let (dx, dy) = (c.x - d.start.0, c.y - d.start.1);
							if (dx * dx + dy * dy).sqrt() <= DRAG_THRESHOLD {
								return;
							}
							d.armed = true;
						}
						let (ox, oy) = root_origin();
						// Reference the moving block's center (ghost top-left + half its size), not the raw
						// pointer — the cell the block visibly covers is where it lands.
						let cx = c.x - d.grab.0 + d.src_w as f64 * STEP / 2.0;
						let cy = c.y - d.grab.1 + d.src_h as f64 * STEP / 2.0;
						d.target = Some(grid.read().resolve_target(cx - ox, cy - oy, STEP, CHROME_H, cols(), d.src_w));
						drag.set(Some(d));
					},
					onpointerup: move |_| {
						let Some(d) = drag() else { return };
						if d.armed {
							if let Some(t) = d.target {
								grid.write().drop(d.source, t, cols());
							}
						} else if let DragSource::Tab { panel, from } = &d.source {
							let mut g = grid.write();
							if let Some(c) = g.cells.iter_mut().find(|c| c.group.id == *from) {
								if let Some(i) = c.group.tabs.iter().position(|p| p == panel) {
									c.group.active = i;
								}
							}
						}
						drag.set(None);
					},
					onpointercancel: move |_| drag.set(None),
				}
			}
			if let Some((title, left, top, gw, gh)) = ghost {
				div { class: "dv-ghost", style: "left:{left}px; top:{top}px; width:{gw}px; height:{gh}px;",
					div { class: "dv-header", div { class: "dv-tab dv-active", "{title}" } }
				}
			}
		}
	}
}
/// The keyboard-driven pane state, shared with the tiles via context: which group the pane ops
/// target (`focused`), and which — if any — is maximized to fill the container (`maximized`, a
/// pure view override that never touches the model).
#[derive(Clone, Copy)]
struct PaneView {
	focused: Signal<Option<GroupId>>,
	maximized: Signal<Option<GroupId>>,
}

/// Linear undo history of settled layouts with a cursor into it. `push` appends a fresh edit,
/// dropping any redo branch ahead of the cursor; `step` walks the cursor by ±1 and returns the
/// snapshot to restore (or `None` at an end).
// ponytail: linear, not a branching tree — two keys (undo/redo) can only express a line. Promote
// to a real tree once there's UI to pick a branch.
#[derive(Default)]
struct History {
	states: Vec<PackedGrid>,
	cursor: usize,
}

impl History {
	fn push(&mut self, g: PackedGrid) {
		if !self.states.is_empty() {
			self.states.truncate(self.cursor + 1);
		}
		self.states.push(g);
		self.cursor = self.states.len() - 1;
	}

	fn step(&mut self, dir: i32) -> Option<PackedGrid> {
		let next = self.cursor.checked_add_signed(dir as isize)?;
		let g = self.states.get(next)?.clone();
		self.cursor = next;
		Some(g)
	}
}

/// An in-flight reposition: what was picked up, where the press began (client px, to measure
/// the [`DRAG_THRESHOLD`]), the live pointer (to drag the ghost naturally), and — once `armed`
/// — the live [`DropTarget`]. `src_w`/`src_h` size both the ghost and a `Pack` target's column
/// clamp; `grab` is the pointer's offset within the picked-up element, so the ghost rides under
/// the same point you grabbed.
#[derive(Clone)]
struct Drag {
	source: DragSource,
	src_w: u32,
	src_h: u32,
	start: (f64, f64),
	grab: (f64, f64),
	cursor: (f64, f64),
	armed: bool,
	target: Option<DropTarget>,
}

/// Corner-resize gesture captured at `pointerdown`: pointer start + the tile's size (in steps) then.
#[derive(Clone, Copy)]
struct ResizeStart {
	px: f64,
	py: f64,
	w: u32,
	h: u32,
}

/// One tile: absolutely positioned at `x*STEP, y*STEP, w*STEP, h*STEP`, with a single header bar
/// (its empty area drags to reposition the whole tile; a tab drags to tear it out — the active
/// tab is the title, so there's no separate titlebar), an empty body filler, and a bottom-right
/// resize grip that snaps the pointer delta to whole steps. The `+`/`x` chrome (right of the
/// tabs): `+` asks the host (via a [`Callback<GroupId>`] context) to open a tab; `x` closes the
/// active tab (and removes the now-empty tile). The body is just a spacer — content rides in the
/// overlay, positioned from the same grid rect. Layout reads come from the preview `view`;
/// gestures write the real grid through [`PackedApi`].
#[component]
fn PackedFrame(idx: usize) -> Element {
	let mut api = use_context::<PackedApi>();
	let panels = use_context::<Signal<Vec<DockPanel>>>();
	let view = use_context::<Memo<PackedGrid>>();
	let mut drag = use_context::<Signal<Option<Drag>>>();
	let PaneView { mut focused, maximized } = use_context::<PaneView>();
	let request_tab = use_context::<Callback<GroupId>>();
	let mut resize = use_signal(|| None::<ResizeStart>);

	let titles: HashMap<PanelId, String> = panels.read().iter().map(|p| (p.id.clone(), p.title.clone())).collect();
	let (gid, x, y, w, h, tabs, active) = {
		let g = view.read();
		let c = &g.cells[idx];
		let tabs: Vec<(PanelId, String)> = c.group.tabs.iter().map(|id| (id.clone(), titles.get(id).cloned().unwrap_or_default())).collect();
		(c.group.id, c.x, c.y, c.w, c.h, tabs, c.group.active)
	};
	// Maximize is a view-only override: the focused tile fills the container (its real grid rect is
	// untouched), every other tile is omitted from the skeleton.
	match *maximized.read() {
		Some(mg) if mg != gid => return rsx! {},
		_ => {}
	}
	let style = if *maximized.read() == Some(gid) {
		"left:0; top:0; width:100%; height:100%;".to_string()
	} else {
		format!("left:{}px; top:{}px; width:{}px; height:{}px;", x as f64 * STEP, y as f64 * STEP, w as f64 * STEP, h as f64 * STEP)
	};

	// While a drag is armed, mark this cell if it's where the source lands (a grey shadow for
	// Displace/Pack) or, for a Tab target, the group whose header is the drop site.
	let (is_shadow, tab_highlight) = match drag.read().clone() {
		Some(d) if d.armed => match d.target {
			Some(DropTarget::Tab(t)) => (false, t == gid),
			Some(DropTarget::Displace { .. }) | Some(DropTarget::Pack { .. }) => {
				let src = match &d.source {
					DragSource::Tile(g) => *g == gid,
					DragSource::Tab { panel, .. } => tabs.iter().any(|(p, _)| p == panel),
				};
				(src, false)
			}
			None => (false, false),
		},
		_ => (false, false),
	};

	// The drag's landing cell: a detail-less grey placeholder snapped to where the floating
	// ghost will commit. No chrome, no content — just the greyed area it points at.
	if is_shadow {
		return rsx! { div { class: "dv-tile dv-shadow", style: "{style}" } };
	}

	let header_class = if tab_highlight { "dv-header dv-tab-drop" } else { "dv-header" };

	rsx! {
		div { class: "dv-tile", style: "{style}",
			div { class: "dv-group",
				div {
					class: "{header_class}",
					onpointerdown: move |e: PointerEvent| {
						if e.trigger_button() != Some(MouseButton::Primary) {
							return;
						}
						e.stop_propagation();
						focused.set(Some(gid));
						let c = e.client_coordinates();
						let g = e.element_coordinates();
						drag.set(Some(Drag { source: DragSource::Tile(gid), src_w: w, src_h: h, start: (c.x, c.y), grab: (g.x, g.y), cursor: (c.x, c.y), armed: false, target: None }));
					},
					for (i , (id , t)) in tabs.iter().enumerate() {
						div {
							key: "{id.0}",
							class: if i == active { "dv-tab dv-active" } else { "dv-tab" },
							onpointerdown: {
								let id = id.clone();
								move |e: PointerEvent| {
									if e.trigger_button() != Some(MouseButton::Primary) {
										return;
									}
									e.stop_propagation();
									focused.set(Some(gid));
									let c = e.client_coordinates();
									let g = e.element_coordinates();
									drag.set(Some(Drag {
										source: DragSource::Tab { panel: id.clone(), from: gid },
										src_w: w,
										src_h: h,
										start: (c.x, c.y),
										grab: (g.x, g.y),
										cursor: (c.x, c.y),
										armed: false,
										target: None,
									}));
								}
							},
							"{t}"
						}
					}
					div { class: "dv-actions",
						button {
							class: "dv-action",
							title: "Add window as a tab",
							onpointerdown: |e: PointerEvent| e.stop_propagation(),
							onclick: move |_| request_tab.call(gid),
							"+"
						}
						button {
							class: "dv-action",
							title: "Close",
							onpointerdown: |e: PointerEvent| e.stop_propagation(),
							onclick: move |_| api.close_active(gid),
							"✕"
						}
					}
				}
				div { class: "dv-content-slot" }
			}
			div {
				class: "dv-resize-handle",
				onpointerdown: move |e: PointerEvent| {
					if e.trigger_button() != Some(MouseButton::Primary) {
						return;
					}
					e.stop_propagation();
					let c = e.client_coordinates();
					resize.set(Some(ResizeStart { px: c.x, py: c.y, w, h }));
					api.grid.write().state = GridState::Resizing;
				},
			}
			if resize().is_some() {
				div {
					style: "position:fixed; inset:0; z-index:1000; cursor:nwse-resize;",
					onpointermove: move |e: PointerEvent| {
						let Some(s) = resize() else { return };
						let c = e.client_coordinates();
						let dw = ((c.x - s.px) / STEP).round() as i64;
						let dh = ((c.y - s.py) / STEP).round() as i64;
						let nw = (s.w as i64 + dw).max(1) as u32;
						let nh = (s.h as i64 + dh).max(1) as u32;
						api.resize(idx, nw, nh);
					},
					onpointerup: move |_| {
						resize.set(None);
						api.grid.write().state = GridState::Settled;
					},
					onpointercancel: move |_| {
						resize.set(None);
						api.grid.write().state = GridState::Settled;
					},
				}
			}
		}
	}
}

/// The flat, id-keyed content overlay. Each panel renders once in a stable list (so Dioxus
/// keeps its component instance — and any inner JS state — alive across restructuring) and is
/// positioned over its host tile's body straight from the (preview) model rect: tile rect minus
/// the fixed chrome, in the packed root's coordinate space. Hidden unless it is its group's
/// active tab.
#[component]
fn PackedContent() -> Element {
	let view = use_context::<Memo<PackedGrid>>();
	let panels = use_context::<Signal<Vec<DockPanel>>>();
	let drag = use_context::<Signal<Option<Drag>>>();
	let maximized = *use_context::<PaneView>().maximized.read();

	// Panels carried by the floating ghost: hidden here so their live content rides the cursor's
	// ghost, not the snapped grey shadow at the landing cell.
	let hidden: HashSet<PanelId> = match drag.read().clone() {
		Some(d) if d.armed => match &d.source {
			DragSource::Tile(g) => view
				.read()
				.cells
				.iter()
				.find(|c| c.group.id == *g)
				.map(|c| c.group.tabs.iter().cloned().collect())
				.unwrap_or_default(),
			DragSource::Tab { panel, .. } => std::iter::once(panel.clone()).collect(),
		},
		_ => HashSet::new(),
	};

	let host: HashMap<PanelId, Slot> = {
		let g = view.read();
		let mut map = HashMap::new();
		for cell in &g.cells {
			let active = cell.group.active_panel();
			for id in &cell.group.tabs {
				map.insert(
					id.clone(),
					Slot {
						group: cell.group.id,
						x: cell.x,
						y: cell.y,
						w: cell.w,
						h: cell.h,
						active: id == active,
					},
				);
			}
		}
		map
	};

	let panels = panels.read();
	rsx! {
		for panel in panels.iter() {
			div {
				key: "{panel.id.0}",
				class: "dv-render-overlay",
				style: if hidden.contains(&panel.id) { "display:none;".to_string() } else { host.get(&panel.id).map(|s| s.style(maximized)).unwrap_or_else(|| "display:none;".into()) },
				{panel.content.clone()}
			}
		}
	}
}

/// Where a panel's content sits: its host group, its host tile's grid rect, and whether it's the
/// active tab.
struct Slot {
	group: GroupId,
	x: u32,
	y: u32,
	w: u32,
	h: u32,
	active: bool,
}

impl Slot {
	/// Inline style placing the content over the tile's body, from the grid rect — the same
	/// math the skeleton uses, so the two cannot drift apart. Inactive tabs stay mounted but
	/// `display:none`. When a group is maximized, its active panel fills the container below the
	/// chrome (matching the skeleton's maximized tile) and every other panel is hidden.
	fn style(&self, maximized: Option<GroupId>) -> String {
		if let Some(mg) = maximized {
			if self.group != mg || !self.active {
				return "display:none;".into();
			}
			return format!("display:block; left:0; top:{CHROME_H}px; width:100%; height:calc(100% - {CHROME_H}px);");
		}
		if !self.active {
			return "display:none;".into();
		}
		let (left, top) = (self.x as f64 * STEP, self.y as f64 * STEP + CHROME_H);
		let (width, height) = (self.w as f64 * STEP, (self.h as f64 * STEP - CHROME_H).max(0.0));
		format!("display:block; left:{left}px; top:{top}px; width:{width}px; height:{height}px;")
	}
}
