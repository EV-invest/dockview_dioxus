//! A group frame: titlebar + tab strip + an empty content slot. Port of
//! `dockview-core/src/dockview/dockviewGroupPanel*` + `tabGroup.ts`, declarative.
//!
//! Crucially the content slot stays **empty** — actual panel content is painted by
//! the [content overlay](super::content) and positioned over this slot's measured
//! box. The frame only contributes chrome and that measured box. (insilicoterminal's
//! `.titlebar` / `.subtitlebar` / `.footerbar` map onto this frame.)

use dioxus::prelude::*;

use crate::{
	api::DockApi,
	model::{Location, gridview::GridNode},
};

/// One pane: titlebar (active panel's title) + tab strip + an empty content slot.
/// Static this phase — titlebar drag, maximize/close handlers, the content slot's
/// measurement `onmounted`, and the 5-zone drop target are Phase 3–4.
#[component]
pub fn GroupFrame(location: Location) -> Element {
	let api = use_context::<DockApi>();
	let title = {
		let model = api.model.read();
		let root = model.grid.as_ref().expect("GroupFrame rendered without a grid");
		let GridNode::Leaf(group) = root.at(&location).expect("GroupFrame: location must resolve") else {
			panic!("GroupFrame: location must point at a leaf");
		};
		model.panels.get(group.active_panel()).expect("active panel has metadata").title.clone()
	};
	rsx! {
		div { class: "dv-group",
			div { class: "dv-titlebar", "{title}" }
			TabStrip { location: location.clone() }
			div { class: "dv-content-slot" }
		}
	}
}

/// The tab strip: one tab per panel in `Group.tabs`, marking the active one. Static —
/// click-to-activate and tab drag are Phase 4.
#[component]
pub fn TabStrip(location: Location) -> Element {
	let api = use_context::<DockApi>();
	let (titles, active) = {
		let model = api.model.read();
		let root = model.grid.as_ref().expect("TabStrip rendered without a grid");
		let GridNode::Leaf(group) = root.at(&location).expect("TabStrip: location must resolve") else {
			panic!("TabStrip: location must point at a leaf");
		};
		let titles: Vec<(String, String)> = group
			.tabs
			.iter()
			.map(|id| (id.0.clone(), model.panels.get(id).expect("tab panel has metadata").title.clone()))
			.collect();
		(titles, group.active)
	};
	rsx! {
		div { class: "dv-tabstrip",
			for (i, (id, title)) in titles.iter().enumerate() {
				div {
					key: "{id}",
					class: if i == active { "dv-tab dv-active" } else { "dv-tab" },
					"{title}"
				}
			}
		}
	}
}
