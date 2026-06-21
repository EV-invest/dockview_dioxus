// Headless CDP probe for the dockview keybind listener. Launches headless chromium against the
// running `dx serve` (insilico) and dispatches chords *synthetically* (bypassing the window
// manager that eats real Alt+<key> presses).
//
// It needs no in-lib diagnostic: it injects its own window keydown listener that runs *after* the
// library's, then reads `event.defaultPrevented`. The library calls `prevent_default()` exactly
// when a bind matches, so `prevented=true` proves the chord hit a binding.
//
//   node examples/kbd_probe.mjs [http://127.0.0.1:54580/]

import { spawn } from 'node:child_process'
import { mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const URL = process.argv[2] ?? 'http://127.0.0.1:54580/'
const PORT = 9333
const MOD = { Alt: 1, Ctrl: 2, Meta: 4, Shift: 8 }
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

const chrome = spawn('chromium', [
	'--headless=new',
	'--no-sandbox',
	`--remote-debugging-port=${PORT}`,
	`--user-data-dir=${mkdtempSync(join(tmpdir(), 'dv-cdp-'))}`,
	URL,
], { stdio: 'ignore' })

async function pageWs() {
	for (let i = 0; i < 50; i++) {
		try {
			const targets = await (await fetch(`http://127.0.0.1:${PORT}/json`)).json()
			const page = targets.find((t) => t.type === 'page' && t.webSocketDebuggerUrl)
			if (page) return page.webSocketDebuggerUrl
		} catch {}
		await sleep(200)
	}
	throw new Error('no page target — chromium did not come up')
}

const ws = new WebSocket(await pageWs())
await new Promise((res) => (ws.onopen = res))

let nextId = 1
const pending = new Map()
const probeLines = []
let lastDialog = null
ws.onmessage = (ev) => {
	const msg = JSON.parse(ev.data)
	if (msg.id && pending.has(msg.id)) {
		pending.get(msg.id)(msg.result)
		pending.delete(msg.id)
	}
	if (msg.method === 'Runtime.consoleAPICalled') {
		const text = msg.params.args.map((a) => a.value ?? '').join(' ')
		if (text.startsWith('PROBE')) probeLines.push(text)
	}
	// The lib's //dbg `alert(...)` shows up here; record + dismiss so the page doesn't block.
	if (msg.method === 'Page.javascriptDialogOpening') {
		lastDialog = msg.params.message
		ws.send(JSON.stringify({ id: nextId++, method: 'Page.handleJavaScriptDialog', params: { accept: true } }))
	}
}
const send = (method, params = {}) =>
	new Promise((res) => {
		const id = nextId++
		pending.set(id, res)
		ws.send(JSON.stringify({ id, method, params }))
	})

await send('Runtime.enable')
await send('Page.enable')
await sleep(3500) // let the wasm bundle boot and install the library's window listener

// Our listener is added on load (first); this one runs after it, so it sees defaultPrevented.
await send('Runtime.evaluate', {
	expression: `window.addEventListener('keydown', e =>
		console.log('PROBE', e.code, 'alt='+e.altKey, 'shift='+e.shiftKey, 'prevented='+e.defaultPrevented))`,
})

async function press(label, { code, key, vk, mods = 0 }) {
	probeLines.length = 0
	lastDialog = null
	await send('Input.dispatchKeyEvent', { type: 'keyDown', code, key, windowsVirtualKeyCode: vk, modifiers: mods })
	await send('Input.dispatchKeyEvent', { type: 'keyUp', code, key, windowsVirtualKeyCode: vk, modifiers: mods })
	await sleep(150)
	console.log(`${label.padEnd(46)} | ${probeLines[0] ?? '(no probe event)'} | dialog=${lastDialog ?? 'none'}`)
}

// Focus a pane by clicking its header centre (sets the lib's `focused` group, which close/maximize need).
const hdr = await send('Runtime.evaluate', {
	expression: `(() => { const r = document.querySelector('.dv-header').getBoundingClientRect(); return JSON.stringify([r.x + r.width/2, r.y + r.height/2]) })()`,
	returnByValue: true,
})
const [hx, hy] = JSON.parse(hdr.result.value)
for (const type of ['mousePressed', 'mouseReleased']) {
	await send('Input.dispatchMouseEvent', { type, x: hx, y: hy, button: 'left', clickCount: 1 })
}
await sleep(100)

// `code` is set to a DELIBERATELY WRONG physical key to simulate a non-QWERTY layout — the lib
// must match on the produced `key`, so prevented=true proves it ignores physical position.
console.log('--- with a pane focused (wrong physical codes on purpose) ---')
await press('key "u"  @ code KeyZ → undo', { code: 'KeyZ', key: 'u', vk: 90 })
await press('key "U"  @ code KeyZ → redo', { code: 'KeyZ', key: 'U', vk: 90, mods: MOD.Shift })
await press('key "f"  @ code KeyP → maximize', { code: 'KeyP', key: 'f', vk: 80 })
await press('key "f"  @ code KeyP → un-maximize', { code: 'KeyP', key: 'f', vk: 80 })
await press('key "?"  @ code Slash → help', { code: 'Slash', key: '?', vk: 191, mods: MOD.Shift })
await press('key "?"  @ code Slash → help off', { code: 'Slash', key: '?', vk: 191, mods: MOD.Shift })
await press('Delete   → close', { code: 'Delete', key: 'Delete', vk: 46 })
await press('key "j"  → unbound (expect prevented=false)', { code: 'KeyJ', key: 'j', vk: 74 })

console.log('--- scope guard: focus an <input>, type into it ---')
await send('Runtime.evaluate', {
	expression: `(() => { const i = document.createElement('input'); document.body.appendChild(i); i.focus(); return document.activeElement.tagName })()`,
	returnByValue: true,
})
await press('u (in <input>) → ignored (expect prevented=false)', { code: 'KeyU', key: 'u', vk: 85 })

ws.close()
chrome.kill()
process.exit(0)
