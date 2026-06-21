//! The Dioxus render layer — everything DOM. Dockview's per-class `this.element`
//! ownership is replaced by declarative `rsx!` derived from the model `Signal`.
//!
//! Two stacked layers, mirroring dockview:
//! 1. the tile *skeleton*: absolutely-positioned frames, tab strips, resize grips. Holds
//!    **no** user content, so restructuring it is harmless.
//! 2. a flat, id-keyed *content overlay* (`OverlayRenderContainer` equivalent): one
//!    absolutely-positioned div per panel. Stable keys ⇒ instances never remount.
//!
//! Both are positioned from the model's integer grid rects (no DOM measuring), so a tile
//! and its content share identical math and cannot drift apart — see [`packed`].

pub mod packed;

use crate::math::Rect;

/// Minimal structural stylesheet. Layout (positioning/sizing) ships with the lib; all
/// colors/sizes read from `--dv-*` custom properties so a host can re-theme without us
/// hardcoding a palette.
pub(crate) const CSS: &str = r#"
.dv-group { display: flex; flex-direction: column; width: 100%; height: 100%;
	background: var(--dv-group-bg, #1e1e1e); }
/* One header bar holds the tabs and the actions (insilico's elevated tab strip); the active
   tab is the title, so there's no separate titlebar. Height is pinned (box-sizing: border-box)
   so the content overlay's fixed chrome offset (CHROME_H in render::packed) matches the skeleton.
   Its empty area is the tile's move-handle; tabs/actions stop propagation for their own gestures. */
.dv-header { flex: 0 0 auto; height: 32px; box-sizing: border-box; display: flex;
	align-items: stretch; overflow: hidden; cursor: grab; background: var(--dv-tabstrip-bg, #2d2d2d); }
.dv-actions { flex: 0 0 auto; margin-left: auto; display: flex; align-items: center; gap: 2px; padding: 0 4px; }
.dv-action { cursor: pointer; border: 0; background: transparent; color: var(--dv-fg, #ddd);
	opacity: 0.55; padding: 0 5px; font: inherit; line-height: 1; }
.dv-action:hover { opacity: 1; background: var(--dv-tab-bg, #2d2d2d); }
.dv-tab { display: flex; align-items: center; padding: 0 14px; font-size: 13px;
	white-space: nowrap; cursor: pointer; background: var(--dv-tab-bg, #2d2d2d);
	border-right: 1px solid var(--dv-tab-border, #1e1e1e); }
.dv-tab.dv-active { background: var(--dv-tab-active-bg, #1e1e1e);
	color: var(--dv-tab-active-fg, #fff); }
.dv-content-slot { flex: 1 1 auto; overflow: hidden; }
.dv-overlay { position: absolute; inset: 0; pointer-events: none; }
.dv-render-overlay { position: absolute; overflow: hidden; pointer-events: auto; }
.dv-resize-handle { position: absolute; right: 0; bottom: 0; width: 14px; height: 14px;
	cursor: nwse-resize; z-index: 101; background: var(--dv-resize-bg, #555); }
.dv-resize-handle::after { content: "⌟"; position: absolute; right: 1px; bottom: -3px;
	font-size: 13px; line-height: 1; color: var(--dv-fg, #ddd); }
.dv-packed { position: relative; width: 100%; height: 100%; overflow: auto;
	color: var(--dv-fg, #ddd); font: 13px/1.4 system-ui, sans-serif; }
.dv-tile { position: absolute; overflow: hidden; background: var(--dv-group-bg, #1e1e1e);
	border: 1px solid var(--dv-tab-border, #333); }
/* Drop feedback: the landing cell drawn as a plain greyed-out area (no chrome, no content),
   the floating ghost that tracks the pointer, and a Tab target's drop site. */
.dv-shadow { background: var(--dv-shadow-bg, rgba(160, 160, 160, 0.18)); border-style: dashed; }
.dv-ghost { position: fixed; z-index: 1001; pointer-events: none; opacity: 0.8; overflow: hidden;
	background: var(--dv-group-bg, #1e1e1e); border: 1px solid var(--dv-accent, #63e9cd);
	box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45); }
.dv-tab-drop { box-shadow: inset 0 0 0 2px var(--dv-accent, #63e9cd); }
"#;

impl From<dioxus::html::geometry::PixelsRect> for Rect {
	fn from(r: dioxus::html::geometry::PixelsRect) -> Self {
		Rect {
			x: r.origin.x,
			y: r.origin.y,
			width: r.size.width,
			height: r.size.height,
		}
	}
}
