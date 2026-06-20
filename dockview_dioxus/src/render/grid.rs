//! The skeleton layer: render the [`GridNode`] tree as nested CSS-flex containers
//! with group frames at the leaves and draggable splitters between siblings. Port of
//! the DOM-building scattered through `gridview.ts`/`branchNode.ts`, but declarative.
//!
//! Recursion is safe here because the skeleton holds no user content — only frames,
//! tab strips, and splitter handles. A branch becomes
//! `div { display:flex; flex-direction: row|column }`, each child a
//! `div { flex-basis: {size}%; min-*: … }`. Browser does the pixel layout.

use dioxus::prelude::*;

use super::group;
use crate::{
	api::DockApi,
	geometry::Orientation,
	model::{Location, gridview::GridNode},
};

/// Render the whole grid from the model in context, or a watermark when empty.
/// (Maximized-leaf-only rendering is Phase 5; we render the full grid for now.)
#[component]
pub fn GridLayer() -> Element {
	let api = use_context::<DockApi>();
	let grid = api.model.read().grid.clone();
	match grid {
		None => rsx! { div { class: "dv-watermark", "No panels" } },
		Some(root) => rsx! { GridNodeView { node: root, location: Vec::new() } },
	}
}

/// Recursively render one node. `location` is its path from the root (passed down so
/// leaves can address themselves into the model).
#[component]
pub fn GridNodeView(node: GridNode, location: Location) -> Element {
	match node {
		GridNode::Leaf(_) => rsx! { group::GroupFrame { location } },
		GridNode::Branch { orientation, children } => {
			let dir = match orientation {
				Orientation::Horizontal => "dv-row",
				Orientation::Vertical => "dv-col",
			};
			// Interleave children with a static splitter gutter between each pair. The
			// interactive `Splitter` (drag -> resize_pair) lands in Phase 4. Keys are by
			// first-descendant group id — cosmetic; skeleton remount is harmless.
			let mut items: Vec<Element> = Vec::new();
			for (i, child) in children.into_iter().enumerate() {
				if i > 0 {
					items.push(rsx! { div { key: "splitter-{i}", class: "dv-splitter" } });
				}
				let mut loc = location.clone();
				loc.push(i);
				let key = child.node.leaves()[0].1.id.0;
				items.push(rsx! {
					div {
						key: "{key}",
						class: "dv-child",
						style: "flex-basis: {child.size}%",
						GridNodeView { node: child.node, location: loc }
					}
				});
			}
			rsx! { div { class: "dv-branch {dir}", {items.into_iter()} } }
		}
	}
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
