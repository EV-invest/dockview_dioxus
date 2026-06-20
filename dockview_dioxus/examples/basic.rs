//! First visual milestone: one tabbed group frame painted from the default layout.
//! Run with `dx serve --example basic --package dockview_dioxus`.

use dioxus::prelude::*;
use dockview_dioxus::{DockArea, DockPanel, PanelId};

fn main() {
	dioxus::launch(app);
}

fn app() -> Element {
	let panels = vec![
		DockPanel { id: PanelId("editor".into()), title: "Editor".into(), content: rsx! { div { "editor content" } } },
		DockPanel { id: PanelId("terminal".into()), title: "Terminal".into(), content: rsx! { div { "terminal content" } } },
		DockPanel { id: PanelId("explorer".into()), title: "Explorer".into(), content: rsx! { div { "explorer content" } } },
	];
	rsx! {
		DockArea { panels, storage_key: None }
	}
}
