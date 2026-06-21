//! The skeleton layer: render the [`GridNode`] tree as nested CSS-flex containers
//! with group frames at the leaves and draggable splitters between siblings. Port of
//! the DOM-building scattered through `gridview.ts`/`branchNode.ts`, but declarative.
//!
//! Recursion is safe here because the skeleton holds no user content — only frames,
//! tab strips, and splitter handles. A branch becomes
//! `div { display:flex; flex-direction: row|column }`, each child a
//! `div { flex-basis: {size}%; min-*: … }`. Browser does the pixel layout.

use std::rc::Rc;

use dioxus::prelude::*;

use super::group;
use crate::{
	api::DockApi,
	geometry::Orientation,
	math::Rect,
	model::{GroupAddr, Location, gridview::{GridNode, resize_branch}},
};

/// Render the grid from the model in context, or a watermark when empty. When a leaf is
/// [maximized](crate::model::DockModel::maximized) only that leaf is rendered (no
/// splitters), filling the whole area — the tree itself is untouched.
#[component]
pub fn GridLayer() -> Element {
	let api = use_context::<DockApi>();
	let (grid, maximized) = {
		let model = api.model.read();
		(model.grid.clone(), model.maximized.clone())
	};
	let Some(root) = grid else {
		return rsx! { div { class: "dv-watermark", "No panels" } };
	};
	match maximized {
		Some(loc) => {
			let node = root.at(&loc).expect("maximized location resolves").clone();
			rsx! { GridNodeView { node, location: loc } }
		}
		None => rsx! { GridNodeView { node: root, location: Vec::new() } },
	}
}

/// Recursively render one node. `location` is its path from the root (passed down so
/// leaves can address themselves into the model).
#[component]
pub fn GridNodeView(node: GridNode, location: Location) -> Element {
	match node {
		GridNode::Leaf(_) => rsx! { group::GroupFrame { addr: GroupAddr::Docked(location) } },
		GridNode::Branch { orientation, children } => {
			let dir = match orientation {
				Orientation::Horizontal => "dv-row",
				Orientation::Vertical => "dv-col",
			};
			// The branch div's measured rect gives the splitters their parent-axis px.
			let mut branch_handle = use_signal(|| None::<Rc<MountedData>>);
			// Interleave children with a draggable splitter gutter between each pair. The
			// gutter before child `i` resizes the pair `i-1`/`i`. Keys are by
			// first-descendant group id — cosmetic; skeleton remount is harmless.
			let mut items: Vec<Element> = Vec::new();
			for (i, child) in children.into_iter().enumerate() {
				if i > 0 {
					items.push(rsx! {
						Splitter { key: "splitter-{i}", parent: location.clone(), index: i - 1, orientation, branch_handle }
					});
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
			rsx! {
				div {
					class: "dv-branch {dir}",
					onmounted: move |e| branch_handle.set(Some(e.data())),
					{items.into_iter()}
				}
			}
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
pub fn Splitter(parent: Location, index: usize, orientation: Orientation, branch_handle: Signal<Option<Rc<MountedData>>>) -> Element {
	let mut api = use_context::<DockApi>();
	let mut drag = use_signal(|| None::<ResizeDrag>);
	let cursor = match orientation {
		Orientation::Horizontal => "col-resize",
		Orientation::Vertical => "row-resize",
	};
	rsx! {
		div {
			class: "dv-splitter",
			onpointerdown: move |e: PointerEvent| {
				let parent = parent.clone();
				async move {
				if e.trigger_button() != Some(dioxus::html::input_data::MouseButton::Primary) {
					return;
				}
				let Some(h) = branch_handle() else { return };
				let Ok(rect) = h.get_client_rect().await else { return };
				let rect: Rect = rect.into();
				let c = e.client_coordinates();
				let (axis_px, start_px) = match orientation {
					Orientation::Horizontal => (rect.width, c.x),
					Orientation::Vertical => (rect.height, c.y),
				};
				let start_sizes = {
					let model = api.model.read();
					let GridNode::Branch { children, .. } = model.grid.as_ref().expect("grid").at(&parent).expect("splitter parent resolves") else {
						panic!("splitter parent must be a branch");
					};
					[children[index].size, children[index + 1].size]
				};
				drag.set(Some(ResizeDrag { parent, index, orientation, start_px, axis_px, start_sizes }));
				}
			},
		}
		if drag().is_some() {
			div {
				style: "position:fixed; inset:0; z-index:1000; cursor:{cursor};",
				onpointermove: move |e: PointerEvent| {
					let Some(d) = drag() else { return };
					let c = e.client_coordinates();
					let cur = match d.orientation {
						Orientation::Horizontal => c.x,
						Orientation::Vertical => c.y,
					};
					let delta_pct = (cur - d.start_px) / d.axis_px * 100.0;
					let mut model = api.model.write();
					// Reset the dragged pair to its press-time sizes so the cumulative delta is
					// absolute, not incremental; then apply through the shared resize path.
					if let GridNode::Branch { children, .. } = model.grid.as_mut().expect("grid").at_mut(&d.parent).expect("splitter parent resolves") {
						children[d.index].size = d.start_sizes[0];
						children[d.index + 1].size = d.start_sizes[1];
					} else {
						panic!("splitter parent must be a branch");
					}
					// REVIEW: de-dups the old inline pair-resize; the fuzzer drives this same path.
					resize_branch(model.grid.as_mut().expect("grid"), &d.parent, d.index, delta_pct);
				},
				onpointerup: move |_| drag.set(None),
				onpointercancel: move |_| drag.set(None),
			}
		}
	}
}
/// In-flight resize captured at `pointerdown`: absolute-from-start (not incremental)
/// to avoid clamp drift. Each `pointermove` resets the pair to `start_sizes` then
/// applies `resize_pair` with the cumulative delta.
#[derive(Clone)]
struct ResizeDrag {
	parent: Location,
	index: usize,
	orientation: Orientation,
	/// Pointer coord along the split axis at `pointerdown`.
	start_px: f64,
	/// Parent branch size along the split axis (px), for delta→percent conversion.
	axis_px: f64,
	/// The two siblings' percentages at `pointerdown`.
	start_sizes: [f64; 2],
}
