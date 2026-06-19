//! The Dioxus render layer — everything DOM. This is where dockview's per-class
//! `this.element` ownership is replaced by declarative `rsx!` derived from the model
//! `Signal`. Dockview infra that Dioxus subsumes is intentionally absent: no
//! `events.ts` Emitter (Signals), no `lifecycle.ts` Disposable (scopes/`use_drop`),
//! no `dom.ts` (rsx).
//!
//! Three stacked layers, painted back-to-front, mirroring dockview:
//! 1. [`grid`]     — recursive *skeleton*: nested flex divs, group frames, splitter
//!    handles. Holds **no** user content, so restructuring it is harmless.
//! 2. [`content`]  — flat, id-keyed *content overlay* (`OverlayRenderContainer`
//!    equivalent): one absolutely-positioned div per panel, positioned from the
//!    measured box of its group's content slot. Stable keys ⇒ instances never remount.
//! 3. [`floating`] / [`drop_overlay`] — floating groups and the live drop indicator.

pub mod content;
pub mod drop_overlay;
pub mod floating;
pub mod grid;
pub mod group;

use std::collections::HashMap;

use dioxus::prelude::*;

use crate::{
	math::Rect,
	model::{DockModel, GroupId},
	panel::DockPanel,
};

/// Measured pixel box of each group's content slot, in container-local coords.
/// Group frames write theirs via `onmounted`/resize; the content overlay reads it to
/// position panels. This is the one place we re-introduce measurement — exactly what
/// dockview's `OverlayRenderContainer` does (`getDomNodePagePosition` + rAF).
pub type GroupBoxes = Signal<HashMap<GroupId, Rect>>;

/// Root component. Owns the `Signal<DockModel>`, provides [`DockApi`](crate::api::DockApi)
/// + [`GroupBoxes`] via context, restores any saved layout, and stacks the three render
/// layers. `#[component]` generates the public `DockAreaProps` from these params.
///
/// - `panels`: the widgets to host; their order here is the stable render order of the
///   content overlay (independent of layout), which is what preserves instances.
/// - `storage_key`: `localStorage` key for autosave/restore; `None` disables persistence.
#[component]
pub fn DockArea(panels: Vec<DockPanel>, storage_key: Option<String>) -> Element {
	// Sketch of the wiring the body will implement:
	//   let model = use_signal(|| restore_or_default(&panels, storage_key.as_deref()));
	//   use_context_provider(|| DockApi { model });
	//   let boxes: GroupBoxes = use_context_provider(|| Signal::new(HashMap::new()));
	//   use_effect(persist-on-change, gated to wasm);
	//   rsx! { div.dv-dockview { GridLayer{} ContentLayer{panels} FloatingLayer{} DropOverlay{} } }
	todo!("own Signal<DockModel>, provide DockApi + GroupBoxes, restore layout, stack the 3 layers")
}

/// Build the initial model: restore from storage if present and valid, else a
/// single-group layout containing the given panels.
pub fn restore_or_default(_panels: &[DockPanel], _storage_key: Option<&str>) -> DockModel {
	todo!("persist::read + serial::load, else default layout from panels")
}
