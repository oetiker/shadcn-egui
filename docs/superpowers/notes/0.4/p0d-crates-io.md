# P0d: crates.io rate limits and crate names

Read on 2026-10-11 (generic User-Agent, read-only requests, 1 request/second).

## Rate limits

Source: `src/rate_limiter.rs` in <https://github.com/rust-lang/crates.io>
(`LimitedAction::default_rate_seconds` and `default_burst`), `main` branch.

| Action          | Burst | Refill            |
|-----------------|-------|-------------------|
| `PublishNew`    | 5     | 1 token / 10 min  |
| `PublishUpdate` | 30    | 1 token / 1 min   |
| `YankUnyank`    | 100   | 1 token / 1 min   |

Caveats:

- These are the code defaults. The production deployment can override them by
  environment variable (`PUBLISH_NEW`, `PUBLISH_UPDATE`) and per user (table
  `publish_rate_overrides`). Neither is visible from outside. The error text on
  the first refusal ("You have published too many new crates in a short period
  of time") includes a retry time; the publish script must read it.
- `https://crates.io/policies` is a client-rendered page and states no numbers.
  The numbers come from the source only.

## Crates to publish: 33

core (1) + `shadcn-egui` (1) + icon crates (5: lucide, tabler, phosphor,
remixicon, hugeicons; all `ship=yes` in `tools/icongen/SETS.tsv`) + font crates
(26; all rows of `tools/fontgen/FONTS.tsv`).

## Minimum publish time for 0.4.0

All 33 are new crates. The first 5 go out at once; the other 28 each wait for
one refill of 10 minutes: 28 x 10 min = 280 min, about **4 h 40 min**. The
publish order (core, then font and icon crates, then `shadcn-egui`) is
unaffected, but `shadcn-egui` publishes last, so it is the 33rd crate.

Later releases publish 33 new versions of existing crates: burst 30, then 3
more at 1 per minute, so about 3 min of waiting.

## Name availability (HTTP status of `GET /api/v1/crates/<name>`)

404 = free. All 33 names returned 404 (free); none is taken.

| Name | Status |
|------|--------|
| shadcn-egui | 404 |
| shadcn-egui-core | 404 |
| shadcn-egui-icons-lucide | 404 |
| shadcn-egui-icons-tabler | 404 |
| shadcn-egui-icons-phosphor | 404 |
| shadcn-egui-icons-remixicon | 404 |
| shadcn-egui-icons-hugeicons | 404 |
| shadcn-egui-font-inter | 404 |
| shadcn-egui-font-noto-sans | 404 |
| shadcn-egui-font-nunito-sans | 404 |
| shadcn-egui-font-figtree | 404 |
| shadcn-egui-font-roboto | 404 |
| shadcn-egui-font-raleway | 404 |
| shadcn-egui-font-dm-sans | 404 |
| shadcn-egui-font-public-sans | 404 |
| shadcn-egui-font-outfit | 404 |
| shadcn-egui-font-jetbrains-mono | 404 |
| shadcn-egui-font-geist | 404 |
| shadcn-egui-font-geist-mono | 404 |
| shadcn-egui-font-lora | 404 |
| shadcn-egui-font-merriweather | 404 |
| shadcn-egui-font-playfair-display | 404 |
| shadcn-egui-font-noto-serif | 404 |
| shadcn-egui-font-roboto-slab | 404 |
| shadcn-egui-font-oxanium | 404 |
| shadcn-egui-font-manrope | 404 |
| shadcn-egui-font-space-grotesk | 404 |
| shadcn-egui-font-montserrat | 404 |
| shadcn-egui-font-ibm-plex-sans | 404 |
| shadcn-egui-font-source-sans-3 | 404 |
| shadcn-egui-font-instrument-sans | 404 |
| shadcn-egui-font-eb-garamond | 404 |
| shadcn-egui-font-instrument-serif | 404 |
