#!/usr/bin/env node
// Measures where headless Chrome breaks the lines of description texts
// (the alert-dialog and alert demo descriptions, and texts whose breaks
// depend on kerning; 14px text on 20px lines, shadcn's `text-sm`) under
// `text-wrap: wrap`, `balance` and `pretty`, at several box widths, for
// every font in tools/fontgen/FONTS.tsv, at the device scales in SCALES,
// and writes crates/shadcn-egui/tests/text_placement/chrome-wraps.tsv.
// The texts have no ligature pairs (fi, ff, tt, ...): the crate draws no
// ligatures (an accepted difference).
//
// A `|` in a text starts or ends a coloured span there (and is no char of
// the text): the text keeps its font across it.
//
// Per row (font, scale, text, width): for each mode the char indices where
// a line after the first starts, comma-separated ("-" for one line). A line
// starts where a char's layout rect (a DOM Range over it) sits lower than
// the char before it.
//
// Each scale runs in its own Chrome with --force-device-scale-factor.
//
// Usage (Node 22+, no packages):
//   node scripts/chrome-text-wrap.mjs <chrome-binary> [<output.tsv>]
// or set CHROME and OUT. Chrome's profiles and the generated pages go to a
// fresh directory under os.tmpdir() (set TMPDIR to move it), removed at
// the end.

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const CHROME = process.argv[2] || process.env.CHROME;
const OUT = process.argv[3] || process.env.OUT ||
  path.join(ROOT, 'crates/shadcn-egui/tests/text_placement/chrome-wraps.tsv');
if (!CHROME) {
  console.error('usage: node scripts/chrome-text-wrap.mjs <chrome-binary> [<output.tsv>]');
  process.exit(2);
}
// Fractional scales put a CSS px off the device pixel grid.
const SCALES = [1, 1.25, 1.5, 1.75, 2];
const MODES = ['wrap', 'balance', 'pretty'];
const WIDTHS = [140, 180, 220, 260, 300, 340];
// Widths off the CSS pixel grid, where a fraction of a pixel decides.
const FRACTIONAL = [96, 113.7, 121, 127.4, 141.1, 150.5, 177, 203];
// Keep in step with TEXTS in crates/shadcn-egui/tests/text_wrap.rs.
const TEXTS = {
  'alert-dialog': ['This action cannot be undone. This will permanently delete your account and remove your data from our servers.', WIDTHS],
  alert: ['Please verify your billing information and try again.', WIDTHS],
  // A word end kerned against the space after it (playfair-display "w ").
  form: ['Use the form below to tell us what went wrong with your order so we can sort it out', FRACTIONAL],
  // Kerning that widens a word end (lora).
  code: ['We have sent you a code by email. Please enter it below to continue with your sign up.', FRACTIONAL],
  // Balance bisection steps of one CSS px (monospace faces).
  save: ['Make changes to your account here. Click save when you are done.', FRACTIONAL],
  // Three words: balance bisects (too few breaks to score); widths where
  // the bisection's last step (one CSS px) decides.
  avatar: ['Select View avatar', [80, 81.2]],
  email: ['Open messages email', [101.9, 103.3, 105.1, 107.3]],
  reset: ['Reset billing email', [62.7, 64.3, 79.6, 80.6, 81.8, 83]],
  // Lines that end after a hyphen inside a word.
  hyphen: ['A well-known step-by-step guide for a long-term follow-up review', FRACTIONAL],
  // Break classes (UAX #14) past spaces and hyphens. At width 1 every
  // break opportunity starts a line.
  dash: ['Wait\u2014what? Pages ten\u2013twenty are missing\u2014again\u2014sadly, said the clerk.', [1, ...FRACTIONAL]],
  zwsp: ['averyveryverylong\u200Bwordwith\u200Bzerowidth\u200Bspaces in it', [1, ...FRACTIONAL]],
  shy: ['Rhein\u00ADwasser\u00ADboots\u00ADkapit\u00E4ns\u00ADm\u00FCtzen\u00ADhalter und Crew', [1, ...FRACTIONAL]],
  digits: ['A well-known state-of-the-art e-mail re-sign 2-3 x-1 pre-1990 test', [1, ...FRACTIONAL]],
  slash: ['Read and/or write to path/to/some/doc or example.com/a/b now.', [1, ...FRACTIONAL]],
  punct: ['Hello (world), [this] is {a} test; it\u2019s okay! Really? Yes: 1,000.50 & 50% less.', [1, ...FRACTIONAL]],
  // CJK: no face of the crate has these glyphs (Chrome falls back to a
  // system face), so only width 1, where the break positions alone decide.
  // Colour changes inside kerned pairs: the kerning stays.
  spans: ['T|oday AV|A W|aves Y|ou T|ry P|ay V|AT Y|ear AW|AY T|yped W|A V|ow L|T', FRACTIONAL],
  cjk: ['\u4E2D\u6587\uFF0C\u65E5\u672C\u8A9E\u300C\u30C6\u30B9\u30C8\u300D\u3067\u3059\u3002\u30AD\u30E3\u30E9\u30FC\u30E1 ab\u4E2Dcd', [1]],
};

function page(fontUrl, weight) {
  const boxes = [];
  for (const [id, [text, widths]] of Object.entries(TEXTS)) {
    for (const w of widths) {
      for (const m of MODES) {
        const html = text.split('|').map((part, i) => (i % 2 ? `<span style="color:#c00">${part}</span>` : part)).join('');
        boxes.push(`<div data-text=${id} data-width=${w} data-mode=${m} style="width:${w}px;text-wrap:${m}">${html}</div>`);
      }
    }
  }
  return `<!doctype html><html><head><meta charset=utf-8><style>
@font-face { font-family: F; src: url('${fontUrl}') format('truetype'); font-weight: ${weight}; font-display: block; }
body { margin: 0; font-family: F; font-size: 14px; line-height: 20px; }
div { margin-bottom: 4px; }
</style></head><body>${boxes.join('')}</body></html>`;
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
  const proc = spawn(CHROME, ['--headless=new', `--force-device-scale-factor=${s}`, '--window-size=600,400', '--no-first-run', '--no-default-browser-check', '--hide-scrollbars',
    '--disable-gpu', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'], { stdio: 'ignore' });
  const portFile = path.join(profile, 'DevToolsActivePort');
  for (let i = 0; i < 300 && !fs.existsSync(portFile); i++) await new Promise((r) => setTimeout(r, 100));
  const [port, wsPath] = fs.readFileSync(portFile, 'utf8').trim().split('\n');
  const cdp = new Cdp(`ws://127.0.0.1:${port}${wsPath}`);
  await cdp.open();
  return { proc, cdp };
}

const MEASURE = `document.fonts.ready.then(() => {
  if (!document.fonts.check('14px F')) throw new Error('font not loaded');
  return [...document.querySelectorAll('div')].map((d) => {
    // Every char of the div's text nodes, in order.
    const chars = [];
    const walk = document.createTreeWalker(d, NodeFilter.SHOW_TEXT);
    for (let t = walk.nextNode(); t; t = walk.nextNode()) {
      for (let j = 0; j < t.length; j++) chars.push([t, j]);
    }
    const r = document.createRange(), starts = [];
    let top = null;
    for (let i = 0; i < chars.length; i++) {
      const [t, j] = chars[i];
      r.setStart(t, j); r.setEnd(t, j + 1);
      // After a soft hyphen, the range over the next char also holds the
      // hyphen drawn at a break, first: the char's own rect is the last.
      const rects = r.getClientRects();
      const prev = chars[i - 1];
      const rect = prev && prev[0].data[prev[1]] === '\u00AD' ? rects[rects.length - 1] : rects[0];
      if (!rect) continue;
      if (top !== null && rect.top > top + 1) starts.push(i);
      top = rect.top;
    }
    return [d.dataset.text, +d.dataset.width, d.dataset.mode, starts.join(',') || '-'];
  });
})`;

async function run(s, tmp) {
  const { proc, cdp } = await launch(tmp, s);
  try {
    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
    const { sessionId: sid } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    await cdp.send('Page.enable', {}, sid);
    const rows = [];
    for (const font of fonts()) {
      const html = path.join(tmp, `${font.id}-${s}.html`);
      fs.writeFileSync(html, page(pathToFileURL(font.file).href, font.weight));
      const loaded = cdp.once('Page.loadEventFired', sid);
      await cdp.send('Page.navigate', { url: pathToFileURL(html).href }, sid);
      await loaded;
      const { result, exceptionDetails } = await cdp.send('Runtime.evaluate', { expression: MEASURE, awaitPromise: true, returnByValue: true }, sid);
      if (exceptionDetails) throw new Error(`${font.id}: ${exceptionDetails.text}`);
      const byKey = new Map();
      for (const [text, width, mode, starts] of result.value) {
        const key = `${text}\t${width}`;
        if (!byKey.has(key)) byKey.set(key, {});
        byKey.get(key)[mode] = starts;
      }
      for (const [key, m] of byKey) rows.push([font.id, s, ...key.split('\t'), ...MODES.map((x) => m[x])]);
    }
    return rows;
  } finally {
    const exited = new Promise((r) => proc.once('exit', r));
    cdp.send('Browser.close').catch(() => {});
    const timer = setTimeout(() => proc.kill(), 5000);
    await exited;
    clearTimeout(timer);
  }
}

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'chrome-wraps-'));
try {
  const rows = [];
  for (const s of SCALES) rows.push(...await run(s, tmp));
  fs.mkdirSync(path.dirname(OUT), { recursive: true });
  fs.writeFileSync(OUT, [['font', 'scale', 'text', 'width', ...MODES].join('\t'), ...rows.map((r) => r.join('\t'))].join('\n') + '\n');
  console.error(`wrote ${rows.length} rows to ${OUT}`);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
