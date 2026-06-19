//! The content overlay — the architectural keystone. Direct analogue of
//! `dockview-core/src/overlay/overlayRenderContainer.ts`.
//!
//! Renders one absolutely-positioned wrapper **per panel**, in a stable, id-keyed
//! list whose order is independent of the layout. Each wrapper is positioned over its
//! group's content slot using the measured [`GroupBoxes`](super::GroupBoxes). Because
//! the key and list position never change when the layout restructures, Dioxus keeps
//! each panel's component instance mounted — only the inline `style` (rect) and
//! `display` (active-tab/visible) change. This is what lets a panel hold live JS state
//! (e.g. a Google Map) while being dragged across the grid.
//!
//! Inactive-tab and off-screen panels render `display:none` (dockview's `'always'`
//! renderer) so their state survives, rather than unmounting.

use dioxus::prelude::*;

use crate::panel::DockPanel;

/// Paint every panel's content into the flat overlay layer.
#[component]
pub fn ContentLayer(panels: Vec<DockPanel>) -> Element {
	// for panel in panels  (STABLE order — never reordered by layout):
	//   let rect = group box that currently hosts panel.id (via model + GroupBoxes)
	//   div { key: "{panel.id.0}", class: "dv-render-overlay",
	//         style: "position:absolute; left/top/width/height from rect; display: …",
	//         {panel.content} }
	todo!("render id-keyed absolutely-positioned wrappers from GroupBoxes; never reorder")
}
