// Headless CDP probe for the dockview keybind listener. Launches headless chromium against the
// running `dx serve` (insilico), dispatches the four chords *synthetically* (so they bypass the
// window manager that eats real Alt+<key> presses), and prints every `DV-KEY ...` the page logs —
// including which bind `matches()` picked. Node 22 built-ins only (global WebSocket + fetch).
//
//   node examples/kbd_probe.mjs [http://127.0.0.1:54580/]
//
// REMOVE with the in-lib `// REMOVE:` diagnostic once the keybind story is settled.

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

// Resolve the page target's debugger websocket.
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
const keyLines = []
ws.onmessage = (ev) => {
	const msg = JSON.parse(ev.data)
	if (msg.id && pending.has(msg.id)) {
		pending.get(msg.id)(msg.result)
		pending.delete(msg.id)
	}
	if (msg.method === 'Runtime.consoleAPICalled') {
		const text = msg.params.args.map((a) => a.value ?? '').join(' ')
		if (text.includes('DV-KEY')) keyLines.push(text)
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
await sleep(3500) // let the wasm bundle boot and install the window listener

// Each chord: keyDown carrying the modifier bitmask, then keyUp. The listener only reads
// code + alt/shift/ctrl, all derived from `modifiers`.
const chords = [
	{ label: 'Alt+Z', code: 'KeyZ', key: 'z', vk: 90, mods: MOD.Alt },
	{ label: 'Alt+Shift+Z', code: 'KeyZ', key: 'Z', vk: 90, mods: MOD.Alt | MOD.Shift },
	{ label: 'Alt+F', code: 'KeyF', key: 'f', vk: 70, mods: MOD.Alt },
	{ label: 'Alt+Delete', code: 'Delete', key: 'Delete', vk: 46, mods: MOD.Alt },
	{ label: 'Ctrl+Z (control)', code: 'KeyZ', key: 'z', vk: 90, mods: MOD.Ctrl },
]
for (const c of chords) {
	await send('Input.dispatchKeyEvent', { type: 'keyDown', code: c.code, key: c.key, windowsVirtualKeyCode: c.vk, modifiers: c.mods })
	await send('Input.dispatchKeyEvent', { type: 'keyUp', code: c.code, key: c.key, windowsVirtualKeyCode: c.vk, modifiers: c.mods })
	await sleep(150)
}
await sleep(300)

console.log('=== DV-KEY lines from page ===')
for (const l of keyLines) console.log(l)
if (!keyLines.length) console.log('(none — wasm listener never logged; bundle stale or not loaded?)')

ws.close()
chrome.kill()
process.exit(0)
