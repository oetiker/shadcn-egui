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

Branch taken: none (every `.crate` at most 8 MB).
Re-measured after the branch: n/a.
