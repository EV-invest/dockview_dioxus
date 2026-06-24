import { expect, test } from '@playwright/test'

// Recognition, not action: the library calls `preventDefault()` the instant a configured chord
// matches (and logs a `dockview keybind:` line), regardless of whether the action then does
// anything. So a probe listener reading `defaultPrevented` proves the window listener saw the key
// — exactly the thing that silently died in Firefox after the MFE switch. A bare unbound key must
// stay un-prevented, or the test would pass on a listener that swallows everything.

test.beforeEach(async ({ page }) => {
	await page.goto('/')
	// The keydown listener is installed when PackedArea mounts; the header proves the wasm booted.
	await page.waitForSelector('.dv-header', { timeout: 30_000 })
	// Probe runs in the bubble phase, after the library's capture-phase listener, so it observes the
	// already-set `defaultPrevented`.
	await page.evaluate(() => {
		globalThis.__dvProbe = []
		window.addEventListener('keydown', (e) => globalThis.__dvProbe.push({ key: e.key, prevented: e.defaultPrevented }))
	})
})

const lastPrevented = (page) => page.evaluate(() => globalThis.__dvProbe.at(-1)?.prevented ?? null)

test('recognizes the undo bind in this browser', async ({ page }) => {
	const logs = []
	page.on('console', (m) => m.text().startsWith('dockview keybind') && logs.push(m.text()))
	await page.keyboard.press('u')
	expect(await lastPrevented(page), 'undo bind must be recognized (preventDefault called)').toBe(true)
	expect(logs.some((l) => l.includes('undo')), 'recognition must log to console').toBe(true)
})

test('recognizes the help bind in this browser', async ({ page }) => {
	await page.keyboard.press('Shift+Slash') // produces "?"
	expect(await lastPrevented(page)).toBe(true)
})

test('leaves an unbound key alone', async ({ page }) => {
	await page.keyboard.press('j')
	expect(await lastPrevented(page), 'an unbound key must not be swallowed').toBe(false)
})
