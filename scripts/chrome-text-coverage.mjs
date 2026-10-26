#!/usr/bin/env node
// Measures how much ink headless Chrome puts into one line of text, in
// light and dark mode, and writes
// crates/shadcn-egui/tests/text_placement/chrome-ink.tsv.
//
// Chrome runs with grayscale anti-aliasing (--disable-lcd-text): egui has
// no subpixel anti-aliasing, and headless Chrome on Linux otherwise
// draws LCD text, whose ink differs.
//
// Cases: the fonts in FONTS (a sans with an optical size axis, a sans
// with a weight axis, a serif, a mono), each at 12, 14, 16 and 20px and
// weights 400, 500 and 600 (line-height 1.5, `font-optical-sizing` left at
// `auto`), at device scale 1 and 2, in the vega neutral palette: light is
// --foreground on --background of `:root`, dark the same of `.dark`.
//
// Per row (font, scale, mode, size, weight): ink = the sum over the
// case's device pixels of |pixel - background| / |foreground -
// background|, on the mean of the three channels of the screenshot pixels
// (sRGB values, as they reach the screen). It counts fully covered pixels
// as 1 and anti-aliased ones by the share of the contrast they show. The
// foreground is read from a swatch of the text colour on the same page.
//
// Each scale runs in its own Chrome with --force-device-scale-factor.
//
// Usage (Node 22+, no packages):
//   node scripts/chrome-text-coverage.mjs <chrome-binary> [<output.tsv>]
// or set CHROME and OUT. Chrome's profiles and the pages go to a fresh
// directory under os.tmpdir() (set TMPDIR to move it), removed at the end.

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
import zlib from 'node:zlib';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const CHROME = process.argv[2] || process.env.CHROME;
const OUT = process.argv[3] || process.env.OUT ||
  path.join(ROOT, 'crates/shadcn-egui/tests/text_placement/chrome-ink.tsv');
if (!CHROME) {
  console.error('usage: node scripts/chrome-text-coverage.mjs <chrome-binary> [<output.tsv>]');
  process.exit(2);
}
// Keep in step with crates/shadcn-egui/tests/text_coverage.rs.
const FONTS = ['inter', 'nunito-sans', 'merriweather', 'jetbrains-mono'];
const SIZES = [12, 14, 16, 20];
const WEIGHTS = [400, 500, 600];
const SCALES = [1, 2];
const TEXT = 'The quick brown fox jumps over 0123 Typography';
const MODES = {
  light: { bg: 'oklch(1 0 0)', fg: 'oklch(0.145 0 0)' },
  dark: { bg: 'oklch(0.145 0 0)', fg: 'oklch(0.985 0 0)' },
};
// Each case is a band of STEP css px; its line box (1.5 x 20px at most)
// and the glyphs' overshoot stay inside it.
const LEFT = 16, TOP = 16, STEP = 44, WIDTH = 760;

function fontFile(id) {
  const dir = path.join(ROOT, `crates/shadcn-egui-font-${id}/fonts`);
  return path.join(dir, fs.readdirSync(dir).filter((n) => n.endsWith('.ttf')).sort()[0]);
}

function cases() {
  const out = [];
  for (const size of SIZES) for (const weight of WEIGHTS) out.push({ size, weight });
  return out;
}

function page(fontUrl, { bg, fg }) {
  const rows = cases().map(({ size, weight }, i) =>
    `<div style="top: ${TOP + i * STEP}px; font-size: ${size}px; font-weight: ${weight}">${TEXT}</div>`);
  const swatch = TOP + cases().length * STEP;
  return `<!doctype html><html><head><meta charset=utf-8><style>
@font-face { font-family: F; src: url('${fontUrl}') format('truetype'); font-weight: 1 1000; font-display: block; }
* { margin: 0; padding: 0; } body { background: ${bg}; }
div { position: absolute; left: ${LEFT}px; font-family: F; line-height: 1.5; color: ${fg}; white-space: pre; }
.swatch { top: ${swatch}px; width: 20px; height: 20px; background: ${fg}; }
</style></head><body>${rows.join('')}<div class=swatch></div></body></html>`;
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
  const proc = spawn(CHROME, ['--headless=new', `--force-device-scale-factor=${s}`, '--window-size=800,700', '--no-first-run', '--no-default-browser-check', '--hide-scrollbars',
    '--disable-gpu', '--disable-lcd-text', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'], { stdio: 'ignore' });
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

async function measure(cdp, sid, font, s, mode, tmp) {
  const html = path.join(tmp, `${font}-${mode}.html`);
  fs.writeFileSync(html, page(pathToFileURL(fontFile(font)).href, MODES[mode]));
  const loaded = cdp.once('Page.loadEventFired', sid);
  await cdp.send('Page.navigate', { url: pathToFileURL(html).href }, sid);
  await loaded;
  const { result } = await cdp.send('Runtime.evaluate', {
    awaitPromise: true, returnByValue: true,
    expression: `document.fonts.ready.then(() => [...document.fonts].map((f) => f.status).join())` }, sid);
  if (result.value !== 'loaded') throw new Error(`${font}: font not loaded (${result.value})`);
  const n = cases().length;
  const height = TOP + n * STEP + 20;
  const shot = await cdp.send('Page.captureScreenshot', {
    format: 'png', clip: { x: 0, y: 0, width: WIDTH, height, scale: 1 } }, sid);
  const img = decodePng(Buffer.from(shot.data, 'base64'));
  if (img.width !== WIDTH * s || img.height !== height * s) throw new Error(`${font} ${s}x: screenshot ${img.width}x${img.height}`);
  const bg = img.at(1, 1);
  const fg = img.at((LEFT + 10) * s, (TOP + n * STEP + 10) * s);
  const [r, g, b] = img.rgb(1, 1);
  if (r !== g || g !== b) throw new Error(`${font} ${s}x ${mode}: background ${r},${g},${b} is not grey`);
  const rows = [];
  cases().forEach(({ size, weight }, i) => {
    let ink = 0, fringe = 0;
    for (let y = (TOP + i * STEP - 6) * s; y < (TOP + (i + 1) * STEP - 6) * s; y++) {
      for (let x = 0; x < img.width; x++) {
        const [pr, pg, pb] = img.rgb(x, y);
        fringe = Math.max(fringe, Math.abs(pr - pb));
        ink += Math.abs((pr + pg + pb) / 3 - bg);
      }
    }
    // Grayscale anti-aliasing leaves no colour fringes.
    if (fringe > 2) throw new Error(`${font} ${s}x ${mode} ${size}px/${weight}: colour fringe ${fringe} (LCD text?)`);
    rows.push([font, s, mode, size, weight, (ink / Math.abs(fg - bg)).toFixed(1)]);
  });
  return rows;
}

async function run(s, tmp) {
  const { proc, cdp } = await launch(tmp, s);
  try {
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
    const { sessionId: sid } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    await cdp.send('Page.enable', {}, sid);
    const { result } = await cdp.send('Runtime.evaluate', { expression: 'devicePixelRatio', returnByValue: true }, sid);
    if (result.value !== s) throw new Error(`devicePixelRatio ${result.value}, wanted ${s}`);
    const rows = [];
    for (const font of FONTS) for (const mode of Object.keys(MODES)) rows.push(...await measure(cdp, sid, font, s, mode, tmp));
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

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'chrome-coverage-'));
try {
  const rows = [];
  for (const s of SCALES) rows.push(...await run(s, tmp));
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, ['font\tscale\tmode\tsize\tweight\tink', ...rows.map((r) => r.join('\t'))].join('\n') + '\n');
  console.error(`wrote ${rows.length} rows to ${OUT}`);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
