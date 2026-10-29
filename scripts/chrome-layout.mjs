#!/usr/bin/env node
// Measures where headless Chrome puts the boxes of a few flex layouts and
// shadcn buttons, at device scale 1, 1.5 and 2, and writes
// crates/shadcn-egui/tests/fixtures/chrome-layout.tsv.
//
// Font: Inter (crates/shadcn-egui-font-inter), 14px text on 20px lines.
// Texts hold no ligature or contextual pair (conformance 2.4).
//
// Cases (CASES below; crates/shadcn-egui/tests/layout_chrome.rs builds the
// same with the crate's helpers):
// - row-text, between-text: a 220px column holding a flex row (gap 8px,
//   items centred; justify-content: space-between for the second) of a
//   flex column (gap 4px) with one paragraph, then a 60x20 box that does
//   not shrink (an egui widget keeps its size): the column shrinks and its
//   text wraps (flex-shrink: 1, min-width: auto).
// - row-two: two such columns and the box in a 300px column: they share
//   the shortfall in proportion to their unwrapped widths.
// - row-min: as row-two in a 180px column, the first column's text one
//   long word: it stops at its min-content and the second takes the rest.
// - cell-text: a flex row of a 100px box that does not shrink (a
//   StripBuilder `Size::exact` cell) holding a flex column with a
//   paragraph, then the 60x20 box: the text wraps at the cell's width.
// - cell-overflow: as cell-text, the cell holding a 150x20 box (an input
//   raised to its content minimum): it overflows the cell, which keeps its
//   width, and the 60x20 box stays after the cell.
// - max-content: the three texts unwrapped (the flex base sizes above).
// - strip-text: a 300px grid of a 100px track and a 1fr track (a
//   StripBuilder `exact` cell and a `remainder` cell), a flex column with
//   a paragraph in each.
// - button-<style>-<content>-<size>: a shadcn button of the vega and the
//   nova style (shadcn-ui registry style-vega.css / style-nova.css and
//   bases/radix/ui/button.tsx, Tailwind v4 defaults expanded below):
//   content text ("Continue"), start (icon, text), both (icon, text,
//   icon) or icon (an icon only, at the icon sizes). The icon is a lucide
//   svg (24 viewBox) sized by the style.
//
// Per row (case, scale, item): the item's x, y, width and height in CSS px
// from the case container's top left, from getBoundingClientRect (Blink's
// LayoutUnit, 1/64 px).
//
// Each scale runs in its own Chrome with --force-device-scale-factor.
//
// Usage (Node 22+, no packages):
//   node scripts/chrome-layout.mjs <chrome-binary> [<output.tsv>]
// or set CHROME and OUT. Chrome's profiles and the page go to a fresh
// directory under os.tmpdir() (set TMPDIR to move it), removed at the end.

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const CHROME = process.argv[2] || process.env.CHROME;
const OUT = process.argv[3] || process.env.OUT ||
  path.join(ROOT, 'crates/shadcn-egui/tests/fixtures/chrome-layout.tsv');
if (!CHROME) {
  console.error('usage: node scripts/chrome-layout.mjs <chrome-binary> [<output.tsv>]');
  process.exit(2);
}
const SCALES = [1, 1.5, 2];
const FONT = path.join(ROOT, 'crates/shadcn-egui-font-inter/fonts/inter.ttf');

// Keep in step with the texts in crates/shadcn-egui/tests/layout_chrome.rs.
const ASK = 'Choose what you want to hear about.';
const MAIL = 'Receive emails about new products.';
const WORD = 'Accessibility';

const p = (id, text) => `<p data-item=${id}>${text}</p>`;
const stack = (id, ...kids) => `<div class=s data-item=${id}>${kids.join('')}</div>`;
const box = `<div class=box data-item=box></div>`;
const CASES = {
  'row-text': [220, `<div class=r>${stack('stack', p('text', ASK))}${box}</div>`],
  'between-text': [220, `<div class="r between">${stack('stack', p('text', ASK))}${box}</div>`],
  'row-two': [300, `<div class=r>${stack('stack-1', p('text-1', ASK))}${stack('stack-2', p('text-2', MAIL))}${box}</div>`],
  'row-min': [180, `<div class=r>${stack('stack-1', p('text-1', WORD))}${stack('stack-2', p('text-2', MAIL))}${box}</div>`],
  'cell-text': [220, `<div class=r><div class=cell data-item=cell>${stack('stack', p('text', ASK))}</div>${box}</div>`],
  'cell-overflow': [220, `<div class=r><div class=cell data-item=cell><div class=wide data-item=wide></div></div>${box}</div>`],
  'max-content': [400, `<div class=s style="align-items:flex-start">${p('text-1', ASK)}${p('text-2', MAIL)}${p('text-3', WORD)}</div>`],
  'strip-text': [300, `<div class=grid>${stack('stack-1', p('text-1', ASK))}${stack('stack-2', p('text-2', MAIL))}</div>`],
};

// lucide "chevron-right" (24 viewBox, stroke 2); the button sizes it.
const ICON = (at) => `<svg data-icon=${at} xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m9 18 6-6-6-6"/></svg>`;
const CONTENT = {
  text: () => 'Continue',
  start: () => `${ICON('inline-start')}Continue`,
  both: () => `${ICON('inline-start')}Continue${ICON('inline-end')}`,
  icon: () => ICON('inline-start'),
};
const BUTTONS = [];
for (const style of ['vega', 'nova']) {
  for (const content of ['text', 'start', 'both']) {
    for (const size of ['sm', 'default', 'lg']) BUTTONS.push([style, content, size]);
  }
  for (const size of ['icon-sm', 'icon', 'icon-lg']) BUTTONS.push([style, 'icon', size]);
}
for (const [style, content, size] of BUTTONS) {
  CASES[`button-${style}-${content}-${size}`] = [400,
    `<button class="button ${style} ${style}-${size}" data-item=button>${CONTENT[content]()}</button>`];
}

// The button classes (cn-button, cn-button-size-*; the button's own
// `inline-flex shrink-0 items-center justify-center whitespace-nowrap`),
// with Tailwind v4's spacing (0.25rem) and text sizes expanded. Only the
// properties that size the box: no colours, rings or radii.
const CSS = `
*,::before,::after { box-sizing: border-box; border: 0 solid; margin: 0; padding: 0; }
@font-face { font-family: F; src: url('${pathToFileURL(FONT).href}') format('truetype'); font-weight: 1 1000; font-display: block; }
html { font-family: F; font-size: 16px; line-height: 1.5; }
body { background: #fff; }
.case { width: var(--w); margin: 0 0 40px 8px; font-size: 14px; line-height: 20px; }
.r { display: flex; align-items: center; gap: 8px; }
.between { justify-content: space-between; }
.s { display: flex; flex-direction: column; gap: 4px; }
.box { min-width: 60px; height: 20px; background: #000; }
.cell { width: 100px; flex-shrink: 0; }
.wide { width: 150px; height: 20px; }
.grid { display: grid; grid-template-columns: 100px 1fr; }
button { font: inherit; letter-spacing: inherit; background: transparent; color: inherit; }
.button { display: inline-flex; flex-shrink: 0; align-items: center; justify-content: center; white-space: nowrap;
  border: 1px solid transparent; background-clip: padding-box; font-size: 14px; line-height: 20px; font-weight: 500; }
.button svg { pointer-events: none; flex-shrink: 0; width: 16px; height: 16px; }
.vega-sm { height: 32px; gap: 4px; padding: 0 10px; }
.vega-sm:has([data-icon=inline-end]) { padding-right: 6px; }
.vega-sm:has([data-icon=inline-start]) { padding-left: 6px; }
.vega-default { height: 36px; gap: 6px; padding: 0 10px; }
.vega-default:has([data-icon=inline-end]), .vega-lg:has([data-icon=inline-end]) { padding-right: 8px; }
.vega-default:has([data-icon=inline-start]), .vega-lg:has([data-icon=inline-start]) { padding-left: 8px; }
.vega-lg { height: 40px; gap: 6px; padding: 0 10px; }
.vega-icon-sm { width: 32px; height: 32px; }
.vega-icon { width: 36px; height: 36px; }
.vega-icon-lg { width: 40px; height: 40px; }
.nova-sm { height: 28px; gap: 4px; padding: 0 10px; font-size: 0.8rem; }
.nova-sm svg { width: 14px; height: 14px; }
.nova-sm:has([data-icon=inline-end]) { padding-right: 6px; }
.nova-sm:has([data-icon=inline-start]) { padding-left: 6px; }
.nova-default { height: 32px; gap: 6px; padding: 0 10px; }
.nova-default:has([data-icon=inline-end]), .nova-lg:has([data-icon=inline-end]) { padding-right: 8px; }
.nova-default:has([data-icon=inline-start]), .nova-lg:has([data-icon=inline-start]) { padding-left: 8px; }
.nova-lg { height: 36px; gap: 6px; padding: 0 10px; }
.nova-icon-sm { width: 28px; height: 28px; }
.nova-icon { width: 32px; height: 32px; }
.nova-icon-lg { width: 36px; height: 36px; }
`;

function page() {
  const cases = Object.entries(CASES).map(([id, [w, html]]) => `<div class=case data-case=${id} style="--w:${w}px">${html}</div>`);
  return `<!doctype html><html><head><meta charset=utf-8><style>${CSS}</style></head><body>${cases.join('')}</body></html>`;
}

const MEASURE = `(async () => {
  await document.fonts.ready;
  if (![...document.fonts].every((f) => f.status === 'loaded')) throw new Error('font not loaded');
  const rows = [];
  for (const c of document.querySelectorAll('[data-case]')) {
    const o = c.getBoundingClientRect();
    for (const e of c.querySelectorAll('[data-item]')) {
      const r = e.getBoundingClientRect();
      rows.push([c.dataset.case, e.dataset.item, r.left - o.left, r.top - o.top, r.width, r.height]);
    }
  }
  return rows;
})()`;

// ---- minimal CDP client over the global WebSocket ----
class Cdp {
  constructor(url) {
    this.ws = new WebSocket(url);
    this.id = 0;
    this.pending = new Map();
    this.waiters = [];
    this.ws.onmessage = (ev) => {
      const m = JSON.parse(ev.data);
      if (m.id && this.pending.has(m.id)) {
        const { resolve, reject } = this.pending.get(m.id);
        this.pending.delete(m.id);
        m.error ? reject(new Error(`${m.error.message} ${m.error.data || ''}`)) : resolve(m.result);
      } else if (m.method) {
        this.waiters = this.waiters.filter((w) => !(w.method === m.method && w.sid === m.sessionId && (w.resolve(m.params), true)));
      }
    };
  }
  open() { return new Promise((res, rej) => { this.ws.onopen = res; this.ws.onerror = rej; }); }
  send(method, params = {}, sessionId) {
    const id = ++this.id;
    this.ws.send(JSON.stringify({ id, method, params, sessionId }));
    return new Promise((resolve, reject) => this.pending.set(id, { resolve, reject }));
  }
  once(method, sid) { return new Promise((resolve) => this.waiters.push({ method, sid, resolve })); }
}
async function launch(tmp, s) {
  const profile = path.join(tmp, `profile-${s}`);
  const proc = spawn(CHROME, ['--headless=new', `--force-device-scale-factor=${s}`, '--window-size=600,400', '--no-first-run', '--no-default-browser-check', '--hide-scrollbars',
    '--disable-gpu', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'], { stdio: 'ignore' });
  const portFile = path.join(profile, 'DevToolsActivePort');
  for (let i = 0; i < 300 && !fs.existsSync(portFile); i++) await new Promise((r) => setTimeout(r, 100));
  const [port, wsPath] = fs.readFileSync(portFile, 'utf8').trim().split('\n');
  const cdp = new Cdp(`ws://127.0.0.1:${port}${wsPath}`);
  await cdp.open();
  return { proc, cdp };
}

async function run(s, tmp) {
  const { proc, cdp } = await launch(tmp, s);
  try {
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
    const { sessionId: sid } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    await cdp.send('Page.enable', {}, sid);
    const html = path.join(tmp, `layout-${s}.html`);
    fs.writeFileSync(html, page());
    const loaded = cdp.once('Page.loadEventFired', sid);
    await cdp.send('Page.navigate', { url: pathToFileURL(html).href }, sid);
    await loaded;
    const { result, exceptionDetails } = await cdp.send('Runtime.evaluate', { expression: MEASURE, awaitPromise: true, returnByValue: true }, sid);
    if (exceptionDetails) throw new Error(`${s}x: ${exceptionDetails.text} ${exceptionDetails.exception?.description || ''}`);
    const n = (v) => +v.toFixed(4);
    return result.value.map(([c, item, x, y, w, h]) => [c, s, item, n(x), n(y), n(w), n(h)]);
  } finally {
    const exited = new Promise((r) => proc.once('exit', r));
    cdp.send('Browser.close').catch(() => {});
    const timer = setTimeout(() => proc.kill(), 5000);
    await exited;
    clearTimeout(timer);
  }
}

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'chrome-layout-'));
try {
  const rows = [];
  for (const s of SCALES) rows.push(...await run(s, tmp));
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, [['case', 'scale', 'item', 'x', 'y', 'width', 'height'].join('\t'), ...rows.map((r) => r.join('\t'))].join('\n') + '\n');
  console.error(`wrote ${rows.length} rows to ${OUT}`);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
