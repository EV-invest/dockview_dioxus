# InsilicoTerminal reference

Reference shots of [insilicoterminal.com](https://insilicoterminal.com)'s panel system — the look the
packed-grid layout engine (`PackedArea`) reproduces.

## Screenshots

> **Drop the PNGs here.** They were pasted into chat and can't be written from there; save them under
> these names so the references below resolve.

### `01-layout.png` — full layout
The overall packed-tile layout. Each pane has:
- an `x` (close) button,
- a `+` button that opens a new window as a **tab** in that same pane,
- a bottom-right resize grip (cursor switches to resize),
- a fixed starting size — panes do **not** fill the whole view; whitespace is left below.

### `02-resize-min-step.png` — minimum resize step
The smallest amount a pane can be shortened by: sizes snap to a fixed step grid, so a resize dragged
between two steps snaps to the nearer side.
