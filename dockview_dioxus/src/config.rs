//! Host-supplied configuration for [`PackedArea`](crate::render::packed::PackedArea).
//! Today just keybinds; passed once as a prop. Matched on physical `code` + modifiers, so a
//! dead-key (`Alt` on some layouts mangling `key()`) can't break a bind.

use dioxus::prelude::*;

/// A single chord: a physical key plus the modifier set that must be held *exactly*.
#[derive(Clone, Copy, PartialEq)]
pub struct Keybind {
	pub code: Code,
	pub alt: bool,
	pub shift: bool,
	pub ctrl: bool,
}

impl Keybind {
	pub(crate) fn matches(&self, e: &KeyboardData) -> bool {
		let m = e.modifiers();
		e.code() == self.code
			&& m.contains(Modifiers::ALT) == self.alt
			&& m.contains(Modifiers::SHIFT) == self.shift
			&& m.contains(Modifiers::CONTROL) == self.ctrl
	}
}

/// Chords acting on the layout / the focused pane. Defaults: `Alt+Z` / `Alt+Shift+Z` for the
/// undo tree, `Alt+Delete` to close the focused pane, `Alt+F` to toggle maximize on it.
#[derive(Clone, Copy, PartialEq)]
pub struct Keybinds {
	pub undo: Keybind = Keybind { code: Code::KeyZ, alt: true, shift: false, ctrl: false },
	pub redo: Keybind = Keybind { code: Code::KeyZ, alt: true, shift: true, ctrl: false },
	pub close: Keybind = Keybind { code: Code::Delete, alt: true, shift: false, ctrl: false },
	pub maximize: Keybind = Keybind { code: Code::KeyF, alt: true, shift: false, ctrl: false },
}

impl Default for Keybinds {
	fn default() -> Self {
		Self { .. }
	}
}

#[derive(Clone, Copy, PartialEq, Default)]
pub struct Config {
	pub keybinds: Keybinds,
}
