//! Host-supplied configuration for [`PackedArea`](crate::render::packed::PackedArea).
//! Today just keybinds; passed once as a prop. Binds match the **produced character**
//! (`KeyboardEvent.key()`), so they follow the user's keyboard layout instead of a hardcoded
//! QWERTY physical position.

/// A single chord: the character the key produces, plus the non-shift modifiers held. Shift is
/// already baked into `key` (`"u"` vs `"U"`), so it isn't a separate flag.
#[derive(Clone, Copy, PartialEq)]
pub struct Keybind {
	/// Matched verbatim against `KeyboardEvent.key()` — e.g. `"u"`, `"U"`, `"Delete"`, `"f"`.
	pub key: &'static str,
	pub alt: bool,
	pub ctrl: bool,
}

impl Keybind {
	#[cfg(target_arch = "wasm32")]
	pub(crate) fn matches(&self, key: &str, alt: bool, ctrl: bool) -> bool {
		self.key == key && self.alt == alt && self.ctrl == ctrl
	}
}

/// Chords acting on the layout / the focused pane. Defaults: `u` / `U` for the undo tree,
/// `Delete` to close the focused pane, `f` to toggle maximize on it, `?` for the keybind hint.
/// They never fire while an editable field is focused (see the listener), so bare letters don't
/// hijack typing.
#[derive(Clone, Copy, PartialEq)]
pub struct Keybinds {
	pub undo: Keybind = Keybind { key: "u", alt: false, ctrl: false },
	pub redo: Keybind = Keybind { key: "U", alt: false, ctrl: false },
	pub close: Keybind = Keybind { key: "Delete", alt: false, ctrl: false },
	pub maximize: Keybind = Keybind { key: "f", alt: false, ctrl: false },
	pub help: Keybind = Keybind { key: "?", alt: false, ctrl: false },
}

impl Default for Keybinds {
	fn default() -> Self {
		Self { .. }
	}
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Config {
	pub keybinds: Keybinds,
}
