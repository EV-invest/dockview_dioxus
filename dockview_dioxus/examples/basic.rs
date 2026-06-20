//! First visual milestone: one tabbed group frame painted from the default layout.
//! Run with `dx serve --example basic --package dockview_dioxus`.

use dioxus::prelude::*;
use dockview_dioxus::{DockApi, DockArea, DockPanel, GroupId, PanelId, math::Rect};

fn main() {
	dioxus::launch(app);
}

fn app() -> Element {
	let panels = vec![
		DockPanel { id: PanelId("editor".into()), title: "Editor".into(), content: rsx! { div { "editor content" } } },
		DockPanel { id: PanelId("terminal".into()), title: "Terminal".into(), content: rsx! { div { "terminal content" } } },
		DockPanel { id: PanelId("explorer".into()), title: "Explorer".into(), content: rsx! { Controls {} } },
	];
	rsx! {
		DockArea { panels, storage_key: Some("dockview-basic".to_string()) }
	}
}

/// Lives inside a panel so it can reach the [`DockApi`] from context — floats whatever
/// group is currently active.
#[component]
fn Controls() -> Element {
	let mut api = use_context::<DockApi>();
	rsx! {
		div {
			"explorer content"
			button {
				// GroupId(0) is the default group minted by `default_layout` (the host would
				// normally hold real group ids; the demo just floats the one it knows).
				onclick: move |_| api.float(GroupId(0), Rect { x: 80.0, y: 80.0, width: 320.0, height: 220.0 }),
				"float this group"
			}
		}
	}
}
