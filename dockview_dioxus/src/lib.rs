#![feature(default_field_values)]
//! `dockview_dioxus` — a dockview-faithful tiling/docking layout for Dioxus.
//!
//! Port of `dockview-core` (MIT, see `docs/refs/dockview-core`). The split that
//! makes it idiomatic Dioxus: dockview fuses a DOM element into every class, we
//! instead keep a pure data **model** ([`model`]) in one `Signal` and derive a
//! declarative **render** ([`render`]) from it. See `docs/ARCHITECTURE.md`.
//!
//! Two render layers, exactly like dockview:
//! - a recursive *skeleton* of frames/tabs/splitters (content-free, safe to remount),
//! - a flat, id-keyed *content overlay* positioned from measured boxes — this is what
//!   keeps panel component instances (and their JS state, e.g. a live map) alive across
//!   layout restructuring. Dockview does the same via `OverlayRenderContainer`.

pub mod api;
pub mod geometry;
pub mod math;
pub mod model;
pub mod panel;
pub mod persist;
pub mod render;

pub use api::DockApi;
pub use geometry::{Orientation, Position};
pub use model::{DockModel, GroupId, PanelId};
pub use panel::DockPanel;
pub use render::{DockArea, DockAreaProps};
