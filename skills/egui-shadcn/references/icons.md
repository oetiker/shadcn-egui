# Icons without an image loader

shadcn uses Lucide icons. Loading them as SVG images would pull `resvg` into
every consumer, so the registry draws them with egui shapes instead:
`components/icon.rs` paints path data, and `icongen` (in this plugin's
`tools/icongen/`) turns SVG files into that data ahead of time. A project
carries only the icons it lists, as plain Rust constants, and gains no runtime
dependency.

## Generate

1. Get the SVGs. Lucide's are in the `lucide-static` npm package, one file per
   icon, and can be fetched individually (pin the version you note in step 2):

   ```bash
   V=1.54.0; mkdir -p assets/lucide
   for n in mic power monitor x; do
     curl -sfL -o assets/lucide/$n.svg https://unpkg.com/lucide-static@$V/icons/$n.svg
   done
   ```

   Any outline icon set works if its icons are square and drawn with strokes.

2. Run the generator from the plugin directory (two levels above this skill):

   ```bash
   cargo run --release --manifest-path <plugin>/tools/icongen/Cargo.toml -- \
     --out src/ui/icons.rs \
     --module-path crate::shadcn::components::icon \
     --provenance "Lucide v1.54.0 (ISC)" \
     assets/lucide/*.svg
   ```

   `--module-path` is where the vendored `icon.rs` lives in the target crate.
   Each SVG becomes a `pub const` named after its file stem (`circle-alert` →
   `CIRCLE_ALERT`), plus `ALL` listing every icon. Re-run it whenever the list
   changes; the output says "do not edit".

3. Keep Lucide's ISC license notice with the generated file or the SVGs.

## Use

```rust
ui.add(IconView::new(&icons::MIC));                       // Metrics.icon, foreground
ui.add(IconView::new(&icons::MIC).size(24.0).color(p.destructive));
ui.add(Button::new("Mute").icon(&icons::MIC));            // icon + text
ui.add(Button::new("Close").icon(&icons::X).size(ButtonSize::Icon)); // icon only
paint_icon(ui.painter(), rect, &icons::MIC, color, t.metrics.icon_stroke); // custom widgets
```

Stroke width is `Metrics.icon_stroke` in icon units (Lucide draws with 2 on a
24 grid; 1.5–1.6 reads lighter at small sizes) and scales with the icon.

## What it does and refuses

- usvg resolves `<circle>`, `<rect rx>`, `<line>`, arcs and transforms into
  lines and Béziers; the rasteriser flattens curves to sub-pixel accuracy.
- Open subpaths get round cap discs at both ends. Lucide draws dots as
  near-zero-length lines (`circle-alert`'s dot is 0.01 units long); without the
  caps they vanish.
- Filled shapes are supported in icon crates (`--crate-dir`): icongen
  flattens them at 0.02 units and triangulates them under their fill rule
  into the crate's blob, together with the boundary contours. The painter
  rasterises each icon (strokes and fills) on the CPU into a cached coverage
  texture, so edges are smooth and an overlapping or faded icon shows no
  darker seam. `--out` modules hold strokes only and reject fills.
- Embedded images, text and non-square view boxes are errors, not silent
  omissions: the generator names the file and stops.
- Works the same under a CPU rasterizer: the texture is built on the CPU and
  drawn as one textured quad.

With the crate, every icon of the theme's sets is a const:
`shadcn_egui::icons::lucide::MIC`, `shadcn_egui::icons::tabler::HOME`
(feature `icons-tabler`). The components draw `Theme.icons` (the slots).

## Icon crates (maintainers)

The crates `shadcn-egui-icons-<lib>` are generated from the pinned npm
packages in `tools/icongen/SETS.tsv`; never edit their `src/`:

```bash
tools/icongen/scripts/generate-crates.sh <scratch dir> [lib...]
```

Each crate holds every icon as a `pub const` (a range into one `icons.bin`
blob), `ALL`, and `SLOTS` for the checkbox tick, select trigger and close
button. Remix Icon stays at its last Apache-2.0 release. After changing
icongen, regenerate the test fixtures:

```bash
cargo run --manifest-path tools/icongen/Cargo.toml --jobs 4 -- \
  --out crates/shadcn-egui/tests/icons/lucide.rs \
  --module-path shadcn_egui::components::icon \
  --provenance "Lucide v1.54.0 (ISC), test fixtures from tools/icongen/tests/fixtures" \
  tools/icongen/tests/fixtures/{mic,circle-alert,settings,x,power,monitor}.svg
cargo run --manifest-path tools/icongen/Cargo.toml --jobs 4 -- \
  --crate-dir crates/shadcn-egui/tests/icons/fixture \
  --slots check=mixed,chevron_down=ring,close=x \
  --provenance "icongen test fixtures (tools/icongen/tests/fixtures); x.svg is Lucide (ISC)" \
  tools/icongen/tests/fixtures/{big-box,mixed,ring,square-hole,star-evenodd,x}.svg
```

`scripts/check-package-size.sh` fails when any crate packages above 8 MB.
