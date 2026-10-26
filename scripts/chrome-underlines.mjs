#!/usr/bin/env node
// Measures where headless Chrome draws `text-decoration: underline` under
// one line of "HHHHHHHH" for every font in tools/fontgen/FONTS.tsv, at
// device scale 1, 1.5 and 2, and writes
// crates/shadcn-egui/tests/text_placement/chrome-underlines.tsv.
//
// Cases (CASES): the text sizes and line heights of the crate parts that
// underline (12px/16px, 14px/20px, 16px/24px; weight 500), each with
// `text-underline-offset: 4px` (Tailwind's `underline-offset-4`, which the
// button and badge link variants set) and with the initial `auto`.
// `text-decoration-thickness` stays `auto`. The text is transparent and the
// underline black, so only the underline paints. Every case sits on whole
// CSS px that are whole device px at each scale.
//
// Per row (font, scale, size, line, offset): baseline = device px from
// the case's top to its baseline (the bottom of a zero-size inline-block
// probe with vertical-align: baseline; the script fails if it is not a
// whole device px); ul_top = the first device px row with underline ink,
// from the same top; ul_cov = the ink of each row from there down, 0 to
// 255 (255 = fully covered), comma separated, from the screenshot pixels
// at the middle column of the text.
//
// Each scale runs in its own Chrome with --force-device-scale-factor.
//
// Usage (Node 22+, no packages):
//   node scripts/chrome-underlines.mjs <chrome-binary> [<output.tsv>]
// or set CHROME and OUT. Chrome's profiles and the pages go to a fresh
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
  path.join(ROOT, 'crates/shadcn-egui/tests/text_placement/chrome-underlines.tsv');
if (!CHROME) {
  console.error('usage: node scripts/chrome-underlines.mjs <chrome-binary> [<output.tsv>]');
  process.exit(2);
}
const SCALES = [1, 1.5, 2];
// Keep in step with crates/shadcn-egui/tests/text_placement.rs.
const CASES = [];
for (const [size, line] of [[12, 16], [14, 20], [16, 24]]) {
  for (const offset of ['4px', 'auto']) CASES.push({ size, line, offset });
}
const TEXT = 'HHHHHHHH', LEFT = 20, TOP = 20, STEP = 40;

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

function page(fontUrl, weight) {
  const cases = CASES.map((c, i) => `<div class=c style="top:${TOP + i * STEP}px;font-size:${c.size}px;line-height:${c.line}px;text-underline-offset:${c.offset}"><span>${TEXT}</span><i class=probe></i></div>`);
  return `<!doctype html><html><head><meta charset=utf-8><style>
@font-face { font-family: F; src: url('${fontUrl}') format('truetype'); font-weight: ${weight}; font-display: block; }
body { margin: 0; background: #fff; }
.c { position: absolute; left: ${LEFT}px; width: 300px; font-family: F; font-weight: 500; color: transparent;
  text-decoration-line: underline; text-decoration-color: #000; white-space: nowrap; }
.probe { display: inline-block; width: 0; height: 0; vertical-align: baseline; }
</style></head><body>${cases.join('')}</body></html>`;
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

async function measure(cdp, sid, font, s, tmp) {
  const html = path.join(tmp, `${font.id}-${s}.html`);
  fs.writeFileSync(html, page(pathToFileURL(font.file).href, font.weight));
  const loaded = cdp.once('Page.loadEventFired', sid);
  await cdp.send('Page.navigate', { url: pathToFileURL(html).href }, sid);
  await loaded;
  const { result } = await cdp.send('Runtime.evaluate', {
    awaitPromise: true, returnByValue: true, expression: `document.fonts.ready.then(() => ({
      cases: [...document.querySelectorAll('.c')].map((c) => {
        const t = c.querySelector('span').getBoundingClientRect(), p = c.querySelector('.probe').getBoundingClientRect();
        return { top: c.getBoundingClientRect().top, x: t.x + t.width / 2, probe: p.bottom };
      }),
      status: [...document.fonts].map((f) => f.status), check: document.fonts.check('14px F') }))` }, sid);
  const v = result.value;
  if (!v.check || v.status.join() !== 'loaded') throw new Error(`${font.id}: font not loaded (${v.status})`);
  const shot = await cdp.send('Page.captureScreenshot', { format: 'png' }, sid);
  const img = decodePng(Buffer.from(shot.data, 'base64'));
  return CASES.map((c, i) => {
    const m = v.cases[i];
    const top = Math.round(m.top * s), x = Math.floor(m.x * s);
    const baseline = (m.probe - m.top) * s;
    if (Math.abs(baseline - Math.round(baseline)) > 1e-3) throw new Error(`${font.id} ${s}x ${c.size}px: baseline ${baseline} off the device pixel`);
    const ink = [];
    for (let y = 0; y < STEP * s; y++) ink.push(255 - Math.round(img.at(x, top + y)));
    const first = ink.findIndex((k) => k > 0);
    if (first < 0) throw new Error(`${font.id} ${s}x ${c.size}px: no underline`);
    let last = first;
    while (last + 1 < ink.length && ink[last + 1] > 0) last++;
    if (ink.slice(last + 1).some((k) => k > 0)) throw new Error(`${font.id} ${s}x ${c.size}px: ink below the underline`);
    return [font.id, s, c.size, c.line, c.offset, Math.round(baseline), first, ink.slice(first, last + 1).join(',')];
  });
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
    for (const font of fonts()) rows.push(...await measure(cdp, sid, font, s, tmp));
    return rows;
  } finally {
    const exited = new Promise((r) => proc.once('exit', r));
    cdp.send('Browser.close').catch(() => {});
    const timer = setTimeout(() => proc.kill(), 5000);
    await exited;
    clearTimeout(timer);
  }
}

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'chrome-underlines-'));
try {
  const rows = [];
  for (const s of SCALES) rows.push(...await run(s, tmp));
  rows.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0) || a[1] - b[1]);
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, [['font', 'scale', 'size', 'line', 'offset', 'baseline', 'ul_top', 'ul_cov'].join('\t'), ...rows.map((r) => r.join('\t'))].join('\n') + '\n');
  console.error(`wrote ${rows.length} rows to ${OUT}`);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
