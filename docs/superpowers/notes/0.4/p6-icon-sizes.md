# P6: icon set sizes (measured before fixing the blob format)

Pinned versions from `tools/icongen/SETS.tsv`; icongen at commit 35f3dc3.

| Set | Version | Icons | Blob bytes (stroke / fill) | .crate bytes | Largest icons |
|---|---|---|---|---|---|
| remixicon | 4.8.0 | 3188 | 4449262 (6376 / 4442886) | 2072453 | java-fill 6840, instagram-line 6414, reactjs-fill 5786, painting-line 5632, baidu-fill 5520 |
| phosphor | 2.1.1 | 1512 | 3277962 (3024 / 3274938) | 1865210 | seal-percent 7102, seal-question 6814, users-four 6200, brain 5964, seal-warning 5860 |
| tabler | 3.49.0 | 5184 | 874078 (822036 / 52042) | 398137 | dice-6 1702, dice-5 1434, dice 1166, dice-4 1166, percentage-100 996 |

Ten largest per set (blob bytes), from the icongen `--measure` output:

- remixicon: java-fill 6840, instagram-line 6414, reactjs-fill 5786, painting-line 5632, baidu-fill 5520, painting-ai-line 5428, copilot-line 5348, baidu-line 5252, java-line 5170, bluesky-line 5124.
- phosphor: seal-percent 7102, seal-question 6814, users-four 6200, brain 5964, seal-warning 5860, flower 5712, seal-check 5618, paw-print 5572, seal 5196, fingerprint 5158.
- tabler: dice-6 1702, dice-5 1434, dice 1166, dice-4 1166, percentage-100 996, percentage-90 960, device-nintendo 948, rosette-discount 938, brand-deliveroo 898, dice-3 898.

All three `.crate` files are far below the 8 MB gate (largest: Remixicon, 2.07 MB).
No icongen error occurred; every icon of every set converted.

Shipped SVGs using attributes that icongen ignores (every fill is encoded solid):

| Set | `fill-opacity` | `opacity` | `stroke-opacity` | `clip-path` / `clipPath` | `mask` |
|---|---|---|---|---|---|
| remixicon | 0 | 0 | 0 | 0 | 0 |
| phosphor | 0 | 0 | 0 | 0 | 0 |
| tabler | 0 | 1 (brand-parsinta) | 0 | 0 | 0 |

Non-uniform paint (fill or stroke other than `currentColor` or `none`), counted over the shipped SVGs:

| Set | Icons with other paint | Names |
|---|---|---|
| remixicon | 2 | remix-run-fill (path fill `#121212`), webhook-line (path fill `#231F20`), both in `Logos/`; the root is `currentColor`, one path overrides it |
| phosphor | 0 | none |
| tabler | 0 | none |

No set uses gradients, `url(#...)` paints, `style` attributes, `<defs>`, `<use>`, `<image>` or white knock-out shapes. icongen encodes the two Remixicon logos in the single theme colour, so they lose their hard-coded dark colour.

Tabler has 49 icons with a non-`none` fill (77 `fill="currentColor"` shapes next to the outline strokes): all are `currentColor`, so none counts as non-uniform. They are brand-steam, stack-back, stack-middle, stack-forward, stack-backward, stack-front, percentage-10 to -100 (20, 25, 30, 33, 40, 50, 60, 66, 70, 75, 80, 90, 100), transform-point-* (4), dice and dice-1 to -6, device-audio-tape, device-nintendo, focus, focus-2, discount, rosette-discount, divide, app-window-* (4), accessible, brand-reddit, brand-deliveroo, yin-yang, outlet, receipt-tax, scuba-diving-tank. The fill-opacity row above stays at 0, so these fills are drawn opaque as intended.

Branch taken: none (every `.crate` at most 8 MB).
Re-measured after the branch: n/a.
