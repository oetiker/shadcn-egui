#!/usr/bin/env node
// Measures where headless Chrome puts the text baseline of "HHHH" in three
// vega-style shadcn elements (default button, badge, text input) and of
// each row of a block wrapped into three rows on fractional lines
// (`text-xs/relaxed`: 12px text, 19.5px lines; items block-1 to block-3) for every
// font in tools/fontgen/FONTS.tsv, at device scale 1, 1.5 and 2, and writes
// crates/shadcn-egui/tests/text_placement/chrome-baselines.tsv.
//
// Per row: line_top = device px from the element's border-box top to the top
// of the line box the text sits in; baseline = device px from the border-box
// top to the bottom edge of the baseline row (the row just below the H ink).
// The baseline comes from the screenshot pixels (bottom edge of the HHHH
// ink) and is checked against the layout baseline (a zero-size inline-block
// probe with vertical-align: baseline next to the element); the script
// fails when the two disagree. line_top comes from the layout: a block span
// around the text for button and badge, the input's inner editor (CDP DOM
// with user-agent shadow roots) for the input.
//
// For the input only, from the screenshot pixels: sel_top and sel_bottom,
// the rows of the selection highlight with all text selected; caret_top,
// caret_bottom and caret_w, the caret's rows and width with the caret after
// the text (device px; "-" for button and badge).
//
// Input: Chrome's inner editor ignores line-height. It is one line of
// line-height normal (ascent + descent + line gap, each rounded to device
// px) centred in the 26px content box, so its top and the baseline land on
// a half device pixel whenever the slack is odd; the text paints on the
// pixel below. The input's line_top is that editor top, rounded up the same
// way. Button and badge positions must be whole device pixels.
//
// Ink rule: a pixel row counts as ink when its summed contrast against the
// element background reaches half the median of the non-empty rows. A
// partial row below the last ink row (antialiased serif bottoms, overshoot)
// would stay below the threshold and not count; the layout check catches a
// font where that rule picks the wrong row. In Chrome 156 the row below the
// H is empty for all 26 fonts, eb-garamond included.
//
// Each scale runs in its own Chrome with --force-device-scale-factor, as on
// a HiDPI screen. Emulation.setDeviceMetricsOverride is not the same: at 2x
// it puts the input baseline 2 device px lower (inter: 48, not 46) and
// paints it 1 px below its own layout probe.
//
// Usage (Node 22+, no packages):
//   node scripts/chrome-text-baselines.mjs <chrome-binary> [<output.tsv>]
// or set CHROME and OUT; SCALES=1,2 measures other device scales (each must
// put the elements on whole device pixels). The output defaults to the committed TSV.
// DEBUG_ROWS=1 also prints per row the editor height and the ink ratios of
// the last row and the row below. Chrome's profiles and the generated pages
// go to a fresh directory under os.tmpdir() (set TMPDIR to move it),
// removed at the end.

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import zlib from 'node:zlib';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const CHROME = process.argv[2] || process.env.CHROME;
const OUT = process.argv[3] || process.env.OUT ||
  path.join(ROOT, 'crates/shadcn-egui/tests/text_placement/chrome-baselines.tsv');
if (!CHROME) {
  console.error('usage: node scripts/chrome-text-baselines.mjs <chrome-binary> [<output.tsv>]');
  process.exit(2);
}
const SCALES = (process.env.SCALES || '1,1.5,2').split(',').map(Number);
const TEXT = 'HHHH';

// Vega style classes (shadcn-ui registry style-vega, Tailwind v4 defaults
// expanded), neutral light theme.
const CSS = `
*,::before,::after { box-sizing: border-box; border: 0 solid; margin: 0; padding: 0; }
:root { --background: oklch(1 0 0); --foreground: oklch(0.145 0 0); --primary: oklch(0.205 0 0);
  --primary-foreground: oklch(0.985 0 0); --input: oklch(0.922 0 0); }
html { line-height: 1.5; font-family: F; -webkit-text-size-adjust: 100%; }
body { background: var(--background); color: var(--foreground); padding: 8px; }
button, input { font: inherit; color: inherit; background: transparent; letter-spacing: inherit; }
.col { display: flex; flex-direction: column; align-items: flex-start; gap: 16px; }
.row { display: block; font-size: 0; line-height: 0; }
.probe { display: inline-block; width: 0; height: 0; vertical-align: baseline; }
.inner { display: block; }
.block { display: block; width: min-content; font-size: 12px; line-height: 19.5px; }
.button { display: inline-flex; flex-shrink: 0; align-items: center; justify-content: center; white-space: nowrap;
  border-radius: 8px; border: 1px solid transparent; background-clip: padding-box;
  font-size: 14px; line-height: 20px; font-weight: 500; height: 36px; gap: 6px; padding: 0 10px;
  background-color: var(--primary); color: var(--primary-foreground); }
.badge { display: inline-flex; width: fit-content; flex-shrink: 0; align-items: center; justify-content: center;
  overflow: hidden; white-space: nowrap; height: 20px; gap: 4px; border-radius: 32px; border: 1px solid transparent;
  padding: 2px 8px; font-size: 12px; line-height: 16px; font-weight: 500;
  background-color: var(--primary); color: var(--primary-foreground); }
.input { width: 200px; min-width: 0; outline: none; height: 36px; border-radius: 8px; border: 1px solid var(--input);
  background: transparent; padding: 4px 10px; font-size: 14px; line-height: 20px; box-shadow: 0 1px 2px 0 rgb(0 0 0 / 0.05);
  caret-color: rgb(255 0 0); }
.input::selection { background: rgb(0 0 255); color: #fff; }
`;

const ITEMS = {
  button: { el: `<button class="m button">${TEXT}</button>`, twin: `<button class="t button"><span class=inner>${TEXT}</span></button>` },
  badge: { el: `<span class="m badge">${TEXT}</span>`, twin: `<span class="t badge"><span class=inner>${TEXT}</span></span>` },
  input: { el: `<input class="m input" value="${TEXT}">` },
  block: { el: `<div class="m block">${TEXT} ${TEXT} ${TEXT}<i class=probe></i></div>` },
};
const BLOCK_ROWS = 3;
const BLOCK_LINE = 19.5;

function page(fontUrl, weight) {
  const rows = [];
  for (const [item, { el, twin }] of Object.entries(ITEMS)) {
    rows.push(`<div class=row data-item=${item} data-kind=m>${el}<i class=probe></i></div>`);
    if (twin) rows.push(`<div class=row data-item=${item} data-kind=t>${twin}<i class=probe></i></div>`);
  }
  return `<!doctype html><html><head><meta charset=utf-8><style>
@font-face { font-family: F; src: url('${fontUrl}') format('truetype'); font-weight: ${weight}; font-display: block; }
${CSS}</style></head><body><div class=col>${rows.join('')}</div></body></html>`;
}

function fonts() {
  const lines = fs.readFileSync(path.join(ROOT, 'tools/fontgen/FONTS.tsv'), 'utf8').trim().split('\n');
  const head = lines[0].split('\t');
  const col = (n) => head.indexOf(n);
  return lines.slice(1).map((l) => {
    const f = l.split('\t');
    const id = f[col('name')];
    const dir = path.join(ROOT, `crates/shadcn-egui-font-${id}/fonts`);
    const ttf = fs.readdirSync(dir).filter((n) => n.endsWith('.ttf')).sort()[0];
    // Static files are listed as weight=path; the first is the regular face.
    const weight = f[col('files')].includes('=') ? f[col('files')].split('=')[0] : '1 1000';
    return { id, file: path.join(dir, ttf), weight };
  });
}

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
  const proc = spawn(CHROME, ['--headless=new', `--force-device-scale-factor=${s}`, '--window-size=400,400', '--no-first-run', '--no-default-browser-check', '--hide-scrollbars',
    '--disable-gpu', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'], { stdio: 'ignore' });
  const portFile = path.join(profile, 'DevToolsActivePort');
  for (let i = 0; i < 300 && !fs.existsSync(portFile); i++) await new Promise((r) => setTimeout(r, 100));
  const [port, wsPath] = fs.readFileSync(portFile, 'utf8').trim().split('\n');
  const cdp = new Cdp(`ws://127.0.0.1:${port}${wsPath}`);
  await cdp.open();
  return { proc, cdp };
}

// ---- PNG decoding (8-bit RGB/RGBA, non-interlaced, as Chrome writes them) ----
function decodePng(buf) {
  let pos = 8, width, height, ctype, idat = [];
  while (pos < buf.length) {
    const len = buf.readUInt32BE(pos), type = buf.toString('ascii', pos + 4, pos + 8);
    const data = buf.subarray(pos + 8, pos + 8 + len);
    if (type === 'IHDR') {
      width = data.readUInt32BE(0); height = data.readUInt32BE(4); ctype = data[9];
      if (data[8] !== 8 || (ctype !== 2 && ctype !== 6) || data[12] !== 0) throw new Error('unsupported PNG');
    } else if (type === 'IDAT') idat.push(data);
    pos += 12 + len;
  }
  const bpp = ctype === 6 ? 4 : 3, stride = width * bpp;
  const raw = zlib.inflateSync(Buffer.concat(idat));
  const px = Buffer.alloc(height * stride);
  for (let y = 0; y < height; y++) {
    const f = raw[y * (stride + 1)], src = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    for (let x = 0; x < stride; x++) {
      const a = x >= bpp ? px[y * stride + x - bpp] : 0, b = y ? px[(y - 1) * stride + x] : 0;
      const c = x >= bpp && y ? px[(y - 1) * stride + x - bpp] : 0;
      let p = src[x];
      if (f === 1) p += a; else if (f === 2) p += b; else if (f === 3) p += (a + b) >> 1;
      else if (f === 4) { const q = a + b - c, pa = Math.abs(q - a), pb = Math.abs(q - b), pc = Math.abs(q - c); p += pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
      px[y * stride + x] = p & 255;
    }
  }
  const rgb = (x, y) => { const i = y * stride + x * bpp; return [px[i], px[i + 1], px[i + 2]]; };
  return { width, height, rgb, at: (x, y) => rgb(x, y).reduce((a, b) => a + b) / 3 };
}

// The box of the pixels in [x0, x1) x [y0, y1) where pred(r, g, b) holds,
// in device px; null when there are none.
function pixelBox(img, x0, x1, y0, y1, pred) {
  let b = null;
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      if (!pred(...img.rgb(x, y))) continue;
      b = b ? { top: Math.min(b.top, y), bottom: Math.max(b.bottom, y + 1), left: Math.min(b.left, x), right: Math.max(b.right, x + 1) }
        : { top: y, bottom: y + 1, left: x, right: x + 1 };
    }
  }
  return b;
}

// Bottom edge of the ink inside [x0, x1) x [y0, y1) in device px, against bg.
function inkBottom(img, x0, x1, y0, y1, bg) {
  const rows = [];
  for (let y = y0; y < y1; y++) {
    let s = 0;
    for (let x = x0; x < x1; x++) s += Math.abs(img.at(x, y) - bg);
    rows.push(s);
  }
  const nz = rows.filter((w) => w > 0).sort((a, b) => a - b);
  if (!nz.length) throw new Error('no ink');
  const thr = 0.5 * nz[nz.length >> 1];
  let last = -1;
  rows.forEach((w, i) => { if (w >= thr) last = i; });
  inkBottom.margin = [rows[last] / (2 * thr), (rows[last + 1] || 0) / (2 * thr)];
  return y0 + last + 1;
}

// CSS px -> device px. Layout positions must sit on whole device pixels; with
// `half`, a half pixel is allowed and rounds up (see the input note above).
const toDev = (v, s, what, half = false) => {
  const d = v * s, r = Math.round(d);
  if (Math.abs(d - r) > 1 / 64 && !(half && Math.abs(d - Math.floor(d) - 0.5) <= 1 / 64)) {
    throw new Error(`${what} not on a device pixel: ${d}`);
  }
  return Math.floor(d + 0.5 + 1 / 64);
};

async function measure(cdp, sid, font, s, tmp) {
  const html = path.join(tmp, `${font.id}.html`);
  fs.writeFileSync(html, page(pathToFileURL(font.file).href, font.weight));
  const loaded = cdp.once('Page.loadEventFired', sid);
  await cdp.send('Page.navigate', { url: pathToFileURL(html).href }, sid);
  await loaded;
  const { result } = await cdp.send('Runtime.evaluate', {
    awaitPromise: true, returnByValue: true, expression: `document.fonts.ready.then(() => {
      const r = (e) => { const b = e.getBoundingClientRect(); return { x: b.x, y: b.y, w: b.width, h: b.height }; };
      const rows = [...document.querySelectorAll('.row')].map((row) => {
        const el = row.firstElementChild, inner = el.querySelector('.inner');
        let text = null;
        if (el.tagName !== 'INPUT') { const g = document.createRange(); g.selectNodeContents(inner || el); text = r(g); }
        return { item: row.dataset.item, kind: row.dataset.kind, el: r(el), probe: r(row.lastElementChild),
                 inner: inner ? r(inner) : null, text };
      });
      return { rows, status: [...document.fonts].map((f) => f.status), check: document.fonts.check('14px F') };
    })` }, sid);
  const v = result.value;
  if (!v.check || v.status.join() !== 'loaded') throw new Error(`${font.id}: font not loaded (${v.status})`);

  // The input's inner editor, from the user-agent shadow root.
  const { root } = await cdp.send('DOM.getDocument', { depth: -1, pierce: true }, sid);
  let input = null;
  const walk = (n) => {
    if (n.nodeName === 'INPUT') input = n;
    for (const c of [...(n.children || []), ...(n.shadowRoots || [])]) walk(c);
  };
  walk(root);
  const editor = input.shadowRoots[0].children.find((c) => c.nodeName === 'DIV');
  const { model } = await cdp.send('DOM.getBoxModel', { nodeId: editor.nodeId }, sid);
  const editorTop = model.border[1];
  const editorHeight = model.border[5] - model.border[1];

  const shot = await cdp.send('Page.captureScreenshot', { format: 'png' }, sid);
  const img = decodePng(Buffer.from(shot.data, 'base64'));

  // The input focused: all text selected, then the caret after the text
  // (it blinks: take the first screenshot that shows it).
  const ev = (expression) => cdp.send('Runtime.evaluate', { expression, returnByValue: true }, sid);
  const inputRow = v.rows.find((r) => r.item === 'input');
  const [ix0, ix1] = [Math.floor(inputRow.el.x * s), Math.ceil((inputRow.el.x + inputRow.el.w) * s)];
  const [iy0, iy1] = [Math.floor(inputRow.el.y * s), Math.ceil((inputRow.el.y + inputRow.el.h) * s)];
  const shoot = async () => decodePng(Buffer.from((await cdp.send('Page.captureScreenshot', { format: 'png' }, sid)).data, 'base64'));
  await ev(`(() => { const i = document.querySelector('input'); i.focus(); i.select(); return 1; })()`);
  await new Promise((r) => setTimeout(r, 100));
  const sel = pixelBox(await shoot(), ix0, ix1, iy0, iy1, (r, g, b) => b > 200 && r < 60 && g < 60);
  await ev(`(() => { const i = document.querySelector('input'); i.setSelectionRange(${TEXT.length}, ${TEXT.length}); return 1; })()`);
  let caret = null;
  for (let k = 0; k < 20 && !caret; k++) {
    await new Promise((r) => setTimeout(r, 100));
    caret = pixelBox(await shoot(), ix0, ix1, iy0, iy1, (r, g, b) => r > 200 && g < 80 && b < 80);
  }
  await ev(`(() => { document.querySelector('input').blur(); return 1; })()`);
  if (!sel || !caret) throw new Error(`${font.id} ${s}x: no selection or caret in the input`);

  const out = [];
  for (const item of Object.keys(ITEMS).filter((i) => i !== 'block')) {
    const m = v.rows.find((r) => r.item === item && r.kind === 'm');
    const t = v.rows.find((r) => r.item === item && r.kind === 't');
    const tag = `${font.id} ${s}x ${item}`;
    const top = toDev(m.el.y, s, `${tag} top`);
    const half = !t;
    const layout = toDev(m.probe.y + m.probe.h, s, `${tag} probe`, half) - top;
    let lineTop;
    if (t) {
      const tl = toDev(t.probe.y + t.probe.h, s, `${tag} twin probe`) - toDev(t.el.y, s, `${tag} twin top`);
      if (tl !== layout) throw new Error(`${tag}: twin baseline ${tl} != ${layout}`);
      lineTop = toDev(t.inner.y - t.el.y, s, `${tag} line top`);
    } else {
      lineTop = toDev(editorTop - m.el.y, s, `${tag} editor top`, half);
    }
    // Ink columns: the text run (for the input: its content box, 10px + 1px border in).
    const ex = m.el.x * s, ew = m.el.w * s, eh = m.el.h * s;
    const [tx0, tx1] = m.text ? [m.text.x * s, (m.text.x + m.text.w) * s] : [ex + 11 * s, ex + ew - 11 * s];
    const bg = img.at(Math.floor(ex + 4 * s), Math.floor(top + eh / 2));
    const ink = inkBottom(img, Math.floor(tx0), Math.ceil(tx1), top + 2 * s, Math.round(top + eh - 2 * s), bg) - top;
    if (ink !== layout) throw new Error(`${tag}: ink baseline ${ink} != layout baseline ${layout}`);
    if (globalThis.debugRows) console.log([font.id, s, item, lineTop, ink, t ? '' : editorHeight * s, ...inkBottom.margin.map((x) => x.toFixed(2))].join('\t'));
    const field = item === 'input'
      ? [sel.top - top, sel.bottom - top, caret.top - top, caret.bottom - top, caret.right - caret.left]
      : ['-', '-', '-', '-', '-'];
    out.push([font.id, s, item, lineTop, ink, ...field]);
  }
  // Block rows: the ink bottom in each line's band; the last row also
  // against its layout probe (the probe after the text).
  const b = v.rows.find((r) => r.item === 'block');
  const btop = toDev(b.el.y, s, `${font.id} ${s}x block top`);
  if (Math.abs(b.el.h - BLOCK_ROWS * BLOCK_LINE) > 1 / 64) throw new Error(`${font.id}: block is ${b.el.h} px tall`);
  const bl = await cdp.send('Runtime.evaluate', { returnByValue: true, expression:
    `(() => { const p = document.querySelector('.block .probe').getBoundingClientRect(); return p.y + p.height; })()` }, sid);
  for (let k = 0; k < BLOCK_ROWS; k++) {
    const y0 = Math.ceil(btop + k * BLOCK_LINE * s), y1 = Math.floor(btop + (k + 1) * BLOCK_LINE * s);
    const ink = inkBottom(img, Math.floor(b.el.x * s), Math.ceil((b.el.x + b.el.w) * s), y0, y1, img.at(2, 2)) - btop;
    if (k === BLOCK_ROWS - 1) {
      // The layout baseline may be fractional; Blink paints it on the nearest row.
      const layout = Math.round(bl.result.value * s) - btop;
      if (ink !== layout) throw new Error(`${font.id} ${s}x block: ink baseline ${ink} != layout baseline ${layout}`);
    }
    out.push([font.id, s, `block-${k + 1}`, '-', ink, '-', '-', '-', '-', '-']);
  }
  return out;
}

async function run(s, tmp) {
  const { proc, cdp } = await launch(tmp, s);
  try {
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
    const { sessionId: sid } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    await cdp.send('Page.enable', {}, sid);
    await cdp.send('DOM.enable', {}, sid);
    await cdp.send('Emulation.setFocusEmulationEnabled', { enabled: true }, sid);
    const { result } = await cdp.send('Runtime.evaluate', { expression: 'devicePixelRatio', returnByValue: true }, sid);
    if (result.value !== s) throw new Error(`devicePixelRatio ${result.value}, wanted ${s}`);
    const rows = [];
    for (const font of fonts()) rows.push(...await measure(cdp, sid, font, s, tmp));
    return rows;
  } finally {
    // Browser.close lets Chrome remove its own temporary files; kill is the fallback.
    const exited = new Promise((r) => proc.once('exit', r));
    cdp.send('Browser.close').catch(() => {});
    const timer = setTimeout(() => proc.kill(), 5000);
    await exited;
    clearTimeout(timer);
  }
}

if (process.env.DEBUG_ROWS) globalThis.debugRows = true;
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'chrome-baselines-'));
try {
  const rows = [];
  for (const s of SCALES) rows.push(...await run(s, tmp));
  rows.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : a[1] - b[1] || (a[2] < b[2] ? -1 : a[2] > b[2] ? 1 : 0)));
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, ['font\tscale\titem\tline_top\tbaseline\tsel_top\tsel_bottom\tcaret_top\tcaret_bottom\tcaret_w', ...rows.map((r) => r.join('\t'))].join('\n') + '\n');
  console.error(`wrote ${rows.length} rows to ${OUT}`);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
