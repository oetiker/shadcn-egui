# P0a: preset codes, React vs svelte

SPEC CONTRADICTION (D13): the two upstream `preset.ts` files differ in value
lists, so one code decodes to different presets on the two sites.

- shadcn-svelte: 493481fa (PIN), `packages/cli/src/preset/preset.ts`.
  shadcn/ui: c2a67849be260701852b0977ba15eb5f3a0ce2a4 (2026-10-10),
  `packages/registry/src/preset/preset.ts` (re-exported by `packages/shadcn/src/preset/index.ts`).
- Lists and PRESET_FIELDS_V1/V2: **differ**. Field order and bit widths
  (V1 3,3,4,6,6,6,6,6 = 40; V2 adds chartColor 6 and fontHeading 5 = 51) are identical.
  Differences:
  1. `PRESET_BASE_COLORS`: React has `gray` at index 3 (neutral, stone, zinc,
     gray, mauve, olive, mist, taupe); svelte has no `gray`
     (neutral, stone, zinc, mauve, olive, mist, taupe). Indices of mauve,
     olive, mist, taupe differ by one.
  2. `PRESET_THEMES` (and so `PRESET_CHART_COLORS`): React has `gray` at index 3;
     svelte has not. Every theme after zinc differs by one (amber..taupe).
  3. `PRESET_ICON_LIBRARIES`: React hardcodes lucide, hugeicons, tabler,
     phosphor, remixicon. Svelte derives it from `keys(iconLibraries)`:
     lucide, tabler, hugeicons, phosphor, remixicon. Indices 1 and 2 swap.
     (React's own `icons/libraries.ts` key order is lucide, tabler, hugeicons,
     phosphor, remixicon, so React's preset list does not follow it either.)
  Identical: PRESET_STYLES, PRESET_FONTS (26), PRESET_FONT_HEADINGS, PRESET_RADII
  keys, PRESET_MENU_ACCENTS, PRESET_MENU_COLORS, V1_CHART_COLOR_MAP content.
  React also exports `PRESET_BASES`/`parsePresetStyle` (not part of the code).
- Icon library key order: lucide, tabler, hugeicons, phosphor, remixicon (svelte) | lucide, hugeicons, tabler, phosphor, remixicon (React).
- V1_CHART_COLOR_MAP: exported by both, applied by **React only**:
  `apps/v4/app/(app)/(create)/lib/preset-query.ts:22,24` and
  `packages/shadcn/src/commands/preset.ts:48` use
  `decoded.chartColor ?? V1_CHART_COLOR_MAP[decoded.theme] ?? decoded.theme`.
  Neither `decodePreset` applies it (an `a` code decodes without chartColor);
  svelte has no use of it outside its definition. Note the React fallback
  is keyed by `theme`, not `baseColor`.
- Decision for `a` codes: open. Follow the React site's consumers
  (chartColor = V1_CHART_COLOR_MAP[theme] ?? theme) or the plan default (index 0)
  depends on which site D13 prefers.
- Codes collected: 11 upstream + 4 hand-made (see tools/stylegen/upstream/preset-codes.txt).

## Evidence that the divergence is real

Decoded with a script holding both list sets (not committed):

| code | svelte lists | React lists |
|---|---|---|
| b4xFeBLg4O (Sera link on both sites) | theme/base neutral | theme/base taupe |
| b2D0wqNxT (Luma link on both) | tabler, theme cyan, chart fuchsia | hugeicons, theme blue, chart emerald |
| b3ZhLl2XfG | hugeicons, theme blue | tabler, theme amber |
| ad3qkJ7 | mist base, mauve theme | olive base, yellow theme |

The svelte checkout at the pin seems to lag React (`gray`, icon order);
svelte's own `presets.ts` lists a taupe Sera-like preset, consistent with React.
This is an inference; only the code comparison above is verified.

## Consequence

Per the decision rule, STOP. D13 needs a ruling: one list set for decoding
(probably React, the newer and canonical site, with svelte at a newer pin
possibly converging), or a per-source decoder. The color palettes of
`gray` also need a base-color table entry if React's lists are chosen.
