//! The skeleton layer: render the [`GridNode`] tree as nested CSS-flex containers
//! with group frames at the leaves and draggable splitters between siblings. Port of
//! the DOM-building scattered through `gridview.ts`/`branchNode.ts`, but declarative.
//!
//! Recursion is safe here because the skeleton holds no user content — only frames,
//! tab strips, and splitter handles. A branch becomes
//! `div { display:flex; flex-direction: row|column }`, each child a
//! `div { flex-basis: {size}%; min-*: … }`. Browser does the pixel layout.

use dioxus::prelude::*;

use crate::model::{Location, gridview::GridNode};

/// Render the whole grid (or the maximized leaf alone) from the model in context.
#[component]
pub fn GridLayer() -> Element {
	todo!("read model.grid from DockApi context; render root via GridNodeView, or just the maximized leaf")
}

/// Recursively render one node. `location` is its path from the root (passed down so
/// leaves/splitters can address themselves for resize and drop).
#[component]
pub fn GridNodeView(node: GridNode, location: Location) -> Element {
	// Branch -> flex container of [child, splitter, child, splitter, …]
	// Leaf   -> group::GroupFrame { location }
	todo!("match node: Branch renders flex children + Splitters; Leaf renders GroupFrame")
}

/// The draggable divider between children `index` and `index+1` of the branch at
/// `parent`. Pointer-drag updates the two siblings' percentages via
/// [`splitview::resize_pair`](crate::model::splitview::resize_pair).
///
/// Pointer pattern (no rich pointer-capture API in Dioxus): on `pointerdown` record
/// the start; render a full-window transparent overlay that owns `pointermove`/`up`
/// for the drag's duration, converting pointer delta against the parent's measured
/// rect into a percentage delta. Same approach the content overlay uses for boxes.
#[component]
pub fn Splitter(parent: Location, index: usize) -> Element {
	todo!("pointerdown -> drag overlay -> resize_pair on move -> commit on up")
}
