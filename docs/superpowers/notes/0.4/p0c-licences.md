# P0c: licences of the preset fonts and the icon sets

All licence statements below were read from the licence file itself (google/fonts blob or file in the npm tarball), not from npm metadata.

## Fonts

Source: google/fonts at commit `bd8f81ddb5c74d5c8897b36ad88b440266245103` (`tools/fontgen/GF_PIN`). All 26 fonts ship (25 OFL-1.1, 1 Apache-2.0); no `ufl/` font; none excluded. The licence header of every `OFL.txt` was read (first lines: copyright + "SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007"); `apache/robotoslab/LICENSE.txt` is the Apache License 2.0 text.

| font | licence | file | variable axes | ships |
|---|---|---|---|---|
| inter | OFL-1.1 | `ofl/inter/Inter[opsz,wght].ttf` | opsz,wght | yes |
| noto-sans | OFL-1.1 | `ofl/notosans/NotoSans[wdth,wght].ttf` | wdth,wght | yes |
| nunito-sans | OFL-1.1 | `ofl/nunitosans/NunitoSans[YTLC,opsz,wdth,wght].ttf` | YTLC,opsz,wdth,wght | yes |
| figtree | OFL-1.1 | `ofl/figtree/Figtree[wght].ttf` | wght | yes |
| roboto | OFL-1.1 | `ofl/roboto/Roboto[wdth,wght].ttf` | wdth,wght | yes |
| raleway | OFL-1.1 | `ofl/raleway/Raleway[wght].ttf` | wght | yes |
| dm-sans | OFL-1.1 | `ofl/dmsans/DMSans[opsz,wght].ttf` | opsz,wght | yes |
| public-sans | OFL-1.1 | `ofl/publicsans/PublicSans[wght].ttf` | wght | yes |
| outfit | OFL-1.1 | `ofl/outfit/Outfit[wght].ttf` | wght | yes |
| jetbrains-mono | OFL-1.1 | `ofl/jetbrainsmono/JetBrainsMono[wght].ttf` | wght | yes |
| geist | OFL-1.1 | `ofl/geist/Geist[wght].ttf` | wght | yes |
| geist-mono | OFL-1.1 | `ofl/geistmono/GeistMono[wght].ttf` | wght | yes |
| lora | OFL-1.1 | `ofl/lora/Lora[wght].ttf` | wght | yes |
| merriweather | OFL-1.1 | `ofl/merriweather/Merriweather[opsz,wdth,wght].ttf` | opsz,wdth,wght | yes |
| playfair-display | OFL-1.1 | `ofl/playfairdisplay/PlayfairDisplay[wght].ttf` | wght | yes |
| noto-serif | OFL-1.1 | `ofl/notoserif/NotoSerif[wdth,wght].ttf` | wdth,wght | yes |
| roboto-slab | Apache-2.0 | `apache/robotoslab/RobotoSlab[wght].ttf` | wght | yes |
| oxanium | OFL-1.1 | `ofl/oxanium/Oxanium[wght].ttf` | wght | yes |
| manrope | OFL-1.1 | `ofl/manrope/Manrope[wght].ttf` | wght | yes |
| space-grotesk | OFL-1.1 | `ofl/spacegrotesk/SpaceGrotesk[wght].ttf` | wght | yes |
| montserrat | OFL-1.1 | `ofl/montserrat/Montserrat[wght].ttf` | wght | yes |
| ibm-plex-sans | OFL-1.1 | `ofl/ibmplexsans/IBMPlexSans[wdth,wght].ttf` | wdth,wght | yes |
| source-sans-3 | OFL-1.1 | `ofl/sourcesans3/SourceSans3[wght].ttf` | wght | yes |
| instrument-sans | OFL-1.1 | `ofl/instrumentsans/InstrumentSans[wdth,wght].ttf` | wdth,wght | yes |
| eb-garamond | OFL-1.1 | `ofl/ebgaramond/EBGaramond[wght].ttf` | wght | yes |
| instrument-serif | OFL-1.1 | `400=ofl/instrumentserif/InstrumentSerif-Regular.ttf` | static (400 only) | yes |

Instrument Serif has only a static Regular (no bold file in google/fonts); its `files` entry is `400=...`.

Reserved Font Names (OFL clause 3: a Modified Version may not use the Reserved Font Name): Raleway, Lora, Merriweather, Playfair Display, IBM Plex ("Plex"), Source Sans 3 ("Source"). Subsetting or instancing in fontgen (Phase 7) makes a Modified Version, so those crates need either unmodified TTFs or a renamed family. See the report concerns.

## Icon sets

| lib | package@version | licence file | SPDX | ship |
|---|---|---|---|---|
| lucide | lucide-static@1.55.0 | `package/LICENSE`: "ISC License ... Lucide Icons and Contributors" | ISC | yes |
| tabler | @tabler/icons@3.49.0 | `package/LICENSE`: "MIT License ... Pawel Kuna" | MIT | yes |
| phosphor | @phosphor-icons/core@2.1.1 | `package/LICENSE`: "MIT License ... Phosphor Icons" | MIT | yes |
| remixicon | remixicon@4.8.0 | `package/License`: Apache License 2.0 (sha256 c71d239d...0ab4) | Apache-2.0 | yes |
| hugeicons | @hugeicons/core-free-icons@4.3.5 | `package/LICENSE.md`: "MIT License, Copyright (c) 2025 Hugeicons" | MIT | yes |

SVG globs checked: lucide 2141 files in `package/icons/`; tabler 5184 in `package/icons/outline/` (also `filled/`); phosphor 1512 in `package/assets/regular/` (also bold, duotone, fill, light, thin); remixicon 3188 in `package/icons/*/*.svg`.

## Remixicon pin

npm versions, newest first: 4.9.1 (2026-01-29), 4.9.0 (2026-01-27), 4.8.0 (2025-12-30), 4.7.0 (2025-10-10).
- 4.9.1 and 4.9.0: `License` file is "Remix Icon License v1.0, Version 1.0 - January 2026" (npm `license` field still says Apache-2.0 - wrong).
- **Pin: 4.8.0** - last release whose `License` file is the Apache License 2.0. **First non-Apache: 4.9.0.**

## Hugeicons verdict

Ship. `LICENSE.md` is the MIT text; the README states "licensed under the MIT License ... use, modify and redistribute them in personal and commercial projects ... keep the MIT notice in LICENSE.md". Pro icons are a separate product and are not in this package.

Data format (`dist/esm/<Name>Icon.js`, one ESM file per icon, 12145 entries incl. maps; `dist/cjs`, `dist/types` also present). `Tick02Icon.js`:

```js
const Tick02Icon = [
  ["path", { d: "M5 14L8.5 17.5L19 6.5", stroke: "currentColor", strokeLinecap: "round", strokeLinejoin: "round", strokeWidth: "1.5", key: "0" }]
];
export { Tick02Icon as default };
```

An array of `[tag, attrs]` pairs, as Phase 6 expects. Attributes are camelCase (`strokeLinecap`, `strokeWidth`) and carry a `key` that must be ignored.
