#![feature(default_field_values)]
//! `dockview_dioxus` — a packed-grid tiling layout for Dioxus.
//!
//! Tiles have a fixed starting size, snap to a step grid, never overlap, and leave
//! whitespace below (InsilicoTerminal's look, `docs/refs/insilico/`). A pure data
//! **model** ([`model`]) lives in one `Signal`; a declarative **render** ([`render`])
//! is derived from it.
//!
//! Two render layers, both positioned from the model's integer grid rects (no DOM measuring):
//! - a *skeleton* of absolutely-positioned tile frames (content-free, safe to remount),
//! - a flat, id-keyed *content overlay* — what keeps panel component instances (and their JS
//!   state) alive across layout restructuring.
//!
//! Tiles reposition by drag: pick up a titlebar or tear a tab, and a cloned `drop` previews
//! the result live (other tiles settling, a shadow over the landing cell) before commit.

pub mod math;
pub mod model;
pub mod panel;
pub mod persist;
pub mod render;

pub use model::{
	Group, GroupId, PanelId,
	packed::{MinSize, PackedGrid, Step},
};
pub use panel::DockPanel;
pub use render::packed::{PackedApi, PackedArea};
