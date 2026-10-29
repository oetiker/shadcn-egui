#!/usr/bin/env node
// Measures how headless Chrome draws a CSS border at device scale 1, 1.5
// and 2, and writes crates/shadcn-egui/tests/fixtures/chrome-borders.tsv.
//
// The page holds one box per border width in WIDTHS: 30px square
// (box-sizing: border-box), a black solid border on every side, 6px
// padding, white inside, and a 4px red square as its content. Every box
// sits on whole CSS px that are whole device px at each scale.
//
// Per row (scale, css border width), from the screenshot pixels along the
// box's middle row and middle column, in device px from the box's outer
// edge: the black run of the left, right, top and bottom border, and the
// offset of the red content from the left and the top edge.
//
// Each scale runs in its own Chrome with --force-device-scale-factor.
//
// Usage (Node 22+, no packages):
//   node scripts/chrome-borders.mjs <chrome-binary> [<output.tsv>]
// or set CHROME and OUT. Chrome's profile and the page go to a fresh
// directory under os.tmpdir() (set TMPDIR to move it), removed at the end.

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import zlib from 'node:zlib';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const CHROME = process.argv[2] || process.env.CHROME;
const OUT = process.argv[3] || process.env.OUT ||
  path.join(ROOT, 'crates/shadcn-egui/tests/fixtures/chrome-borders.tsv');
if (!CHROME) {
  console.error('usage: node scripts/chrome-borders.mjs <chrome-binary> [<output.tsv>]');
  process.exit(2);
}
const SCALES = [1, 1.5, 2];
// Keep in step with WIDTHS in crates/shadcn-egui/tests/borders.rs.
const WIDTHS = [0.5, 1, 1.5, 2, 3];
const SIZE = 30, PAD = 6, STEP = 40, LEFT = 20, TOP = 20;

function page() {
  const boxes = WIDTHS.map((w, i) => `<div class=b style="left:${LEFT + i * STEP}px;border-width:${w}px"><div class=c></div></div>`);
  return `<!doctype html><html><head><meta charset=utf-8><style>
body { margin: 0; background: #fff; }
.b { position: absolute; top: ${TOP}px; width: ${SIZE}px; height: ${SIZE}px; box-sizing: border-box;
  border-style: solid; border-color: #000; padding: ${PAD}px; background: #fff; }
.c { width: 4px; height: 4px; background: #f00; }
</style></head><body>${boxes.join('')}</body></html>`;
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
  const proc = spawn(CHROME, ['--headless=new', `--force-device-scale-factor=${s}`, '--window-size=600,400', '--no-first-run', '--no-default-browser-check', '--hide-scrollbars',
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

// Runs of pixels where pred holds along a line of n pixels, as
// [first, length] pairs.
function runs(n, pred) {
  const out = [];
  for (let i = 0; i < n; i++) {
    if (!pred(i)) continue;
    if (out.length && out[out.length - 1][0] + out[out.length - 1][1] === i) out[out.length - 1][1]++;
    else out.push([i, 1]);
  }
  return out;
}

async function run(s, tmp) {
  const { proc, cdp } = await launch(tmp, s);
  try {
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
    const { sessionId: sid } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    await cdp.send('Page.enable', {}, sid);
    const html = path.join(tmp, `borders-${s}.html`);
    fs.writeFileSync(html, page());
    const loaded = cdp.once('Page.loadEventFired', sid);
    await cdp.send('Page.navigate', { url: pathToFileURL(html).href }, sid);
    await loaded;
    const shot = await cdp.send('Page.captureScreenshot', { format: 'png' }, sid);
    const img = decodePng(Buffer.from(shot.data, 'base64'));
    const black = ([r, g, b]) => r + g + b < 3 * 128;
    const red = ([r, g, b]) => r > 200 && g < 80 && b < 80;
    const rows = [];
    WIDTHS.forEach((w, i) => {
      const x0 = (LEFT + i * STEP) * s, y0 = TOP * s, n = SIZE * s;
      const midY = y0 + Math.floor(n / 2), midX = x0 + Math.floor(n / 2);
      const across = runs(n, (k) => black(img.rgb(x0 + k, midY)));
      const down = runs(n, (k) => black(img.rgb(midX, y0 + k)));
      if (across.length !== 2 || down.length !== 2) throw new Error(`${s}x ${w}px: border runs ${JSON.stringify([across, down])}`);
      const [l, r] = across, [t, b] = down;
      if (l[0] !== 0 || t[0] !== 0 || r[0] + r[1] !== n || b[0] + b[1] !== n) throw new Error(`${s}x ${w}px: border off the box edge`);
      const cx = runs(n, (k) => red(img.rgb(x0 + k, y0 + Math.floor((PAD + 2) * s) + t[1])));
      const cy = runs(n, (k) => red(img.rgb(x0 + Math.floor((PAD + 2) * s) + l[1], y0 + k)));
      if (!cx.length || !cy.length) throw new Error(`${s}x ${w}px: no content`);
      rows.push([s, w, l[1], r[1], t[1], b[1], cx[0][0], cy[0][0]]);
    });
    return rows;
  } finally {
    const exited = new Promise((r) => proc.once('exit', r));
    cdp.send('Browser.close').catch(() => {});
    const timer = setTimeout(() => proc.kill(), 5000);
    await exited;
    clearTimeout(timer);
  }
}

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'chrome-borders-'));
try {
  const rows = [];
  for (const s of SCALES) rows.push(...await run(s, tmp));
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, [['scale', 'border', 'left', 'right', 'top', 'bottom', 'content_x', 'content_y'].join('\t'), ...rows.map((r) => r.join('\t'))].join('\n') + '\n');
  console.error(`wrote ${rows.length} rows to ${OUT}`);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
