# Phase 6 — Icons: filled icons, blob storage, icon crates

> Part of [00-index.md](00-index.md). Read its Global Constraints first, and
> section D ("Icons (Phase 6)") of "Cross-phase interfaces and design notes":
> it is the contract for this phase.

**Spec:** D14, D18, §6.2 (icon slots, blob storage), §6.3 (filled icons,
Remixicon pin, Hugeicons licence), §7 (icon snapshots, zoomed fill edge).
**Cites:** `tools/icongen/SETS.tsv` and `docs/superpowers/notes/0.4/p0c-licences.md`
(Phase 0 Task 3: versions, licences, the Remixicon pin, the Hugeicons verdict
and data format).

**Outcome:** icongen turns filled SVG paths into pre-triangulated meshes
(lyon, tool-only) with outward miters per contour vertex, and writes whole
icon sets as one binary blob per crate (`--crate-dir`). `Icon` holds an
`IconShape` (`Segs` or `Blob`); core decodes blobs without dependencies.
`paint_icon` draws the fill as one `egui::Mesh` with an outward-only
one-pixel fringe, then the strokes. Crates `shadcn-egui-icons-lucide`
(normal dependency, full set) and `shadcn-egui-icons-{tabler,phosphor,remixicon}`
(+ `hugeicons` if `SETS.tsv` says `ship=yes`) export every icon, `ALL` and
`SLOTS`; `Theme::new` uses Lucide's `SLOTS`; features `icons-<lib>` and
`icons-all`; `scripts/check-package-size.sh` fails above 8 MB.

Order matters: Task 6.2 measures the full Remixicon and Phosphor sets
**before** Task 6.3 fixes the format in the core decoder. Existing snapshots
must not change in this phase (the three slot icons have integer coordinates,
so quantisation leaves them exact). New PNGs appear in Task 6.4 (zoomed fill
edges) and Task 6.6 (slot snapshots); each has a user review stop.

Throughout: `$WT`, `$SCRATCH` and `$UP` as in the index Setup; run every
command from `$WT`.

---

### Task 6.1: icongen fill support, blob encoder, `--crate-dir`, `--measure`

**Files:**
- Modify: `tools/icongen/Cargo.toml` (add `lyon_tessellation`)
- Create: `tools/icongen/src/fill.rs`
- Create: `tools/icongen/src/blob.rs`
- Modify: `tools/icongen/src/lib.rs`
- Modify: `tools/icongen/src/main.rs`
- Create: `tools/icongen/tests/fixtures/{square-hole,star-evenodd,star-nonzero,ring,big-box,mixed,overlap,far}.svg`
- Create: `tools/icongen/tests/fill.rs`
- Modify: `tools/icongen/tests/convert.rs` (`fills_are_rejected` becomes a stroke-only check)

**Interfaces:**
- Consumes: usvg 0.48 (`Path::fill()`, `Fill::rule()`, `usvg::FillRule`), `lyon_tessellation` 1.x.
- Produces (crate `icongen`, tool-only):
  - `pub struct Converted { pub name: String, pub size: f32, pub segs: Vec<Seg>, pub fill: Option<Fill> }` — `segs` in the SVG's own units (side `size`), `fill` already in the 24-unit box and quantised.
  - `pub fn convert(name: &str, svg: &str) -> Result<Converted, String>` — fills are no longer an error.
  - `pub fn ensure_stroke_only(icon: &Converted) -> Result<(), String>` — the `--out` mode check.
  - `fill::{FillRule, FillPath, Fill, FLATTEN_TOLERANCE, flatten, build, inside, miter}`:
    `pub enum FillRule { NonZero, EvenOdd }`,
    `pub struct FillPath { pub rule: FillRule, pub contours: Vec<Vec<[f32; 2]>> }`,
    `pub struct Fill { pub vertices: Vec<[f32; 2]>, pub indices: Vec<u16>, pub contours: Vec<Vec<([f32; 2], [f32; 2])>> }` (point, outward miter; a zero miter marks a vertex whose edges have fill on both sides),
    `pub const FLATTEN_TOLERANCE: f32 = 0.02`.
  - `blob::{COORD_SCALE, MITER_SCALE, quantise, encode, decode, stroke_bytes}`:
    `pub fn encode(icon: &Converted) -> Result<Vec<u8>, String>` (one record, format of index D),
    `pub fn decode(bytes: &[u8]) -> (Vec<Seg>, Option<Fill>)` (test reference decoder).
  - **NEW INTERFACE:** `pub struct Slots { pub check: String, pub chevron_down: String, pub close: String }`, `pub fn parse_slots(s: &str) -> Result<Slots, String>`, `pub fn render_crate(icons: &[Converted], slots: &Slots, provenance: &str) -> Result<(String, Vec<u8>), String>` (lib.rs text, blob bytes; icons sorted by name), `pub fn measure(icons: &[Converted]) -> Result<String, String>`.
  - CLI: `--out FILE [--module-path PATH] [--provenance TEXT] SVG…` (unchanged, errors on fills); `--crate-dir DIR --slots check=<icon>,chevron_down=<icon>,close=<icon> [--provenance TEXT] SVG…`; `--measure SVG…` (may be combined with `--crate-dir`).

- [ ] **Step 1: Add the fixtures**

`tools/icongen/tests/fixtures/square-hole.svg` (even-odd hole):

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill-rule="evenodd" d="M4 4H20V20H4Z M9 9H15V15H9Z"/></svg>
```

`tools/icongen/tests/fixtures/star-evenodd.svg`:

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill-rule="evenodd" d="M12 2 L18 21 L2 9 L22 9 L6 21 Z"/></svg>
```

`tools/icongen/tests/fixtures/star-nonzero.svg`:

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path fill-rule="nonzero" d="M12 2 L18 21 L2 9 L22 9 L6 21 Z"/></svg>
```

`tools/icongen/tests/fixtures/ring.svg` (circle clockwise, square hole
counter-clockwise: a hole under the non-zero rule, with curved edges):

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M12 2a10 10 0 1 1 0 20a10 10 0 1 1 0-20Z M8 8V16H16V8Z"/></svg>
```

`tools/icongen/tests/fixtures/big-box.svg` (Phosphor-style 256 view box, no width/height):

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><rect x="32" y="32" width="192" height="192"/></svg>
```

`tools/icongen/tests/fixtures/mixed.svg` (Tabler/Hugeicons style: a stroke plus a filled dot):

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2"><path d="M4 12h16"/><circle cx="12" cy="6" r="2" fill="black" stroke="none"/></svg>
```

`tools/icongen/tests/fixtures/overlap.svg` (two filled paths that overlap):

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M2 2H20V20H2Z"/><path d="M8 8H22V22H8Z"/></svg>
```

`tools/icongen/tests/fixtures/far.svg` (a coordinate outside −32..32 after normalisation):

```xml
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="black"><path d="M0 0L40 0"/></svg>
```

- [ ] **Step 2: Write the failing tests**

`tools/icongen/tests/fill.rs`:

```rust
use icongen::blob::{decode, encode, COORD_SCALE, MITER_SCALE};
use icongen::{convert, measure, parse_slots, render_crate, Fill, Seg};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/fixtures/{name}.svg", env!("CARGO_MANIFEST_DIR"))).expect("fixture")
}

fn fill_of(name: &str) -> Fill {
    convert(name, &fixture(name)).expect("converts").fill.expect("has a fill")
}

fn tri(f: &Fill, t: &[u16]) -> [[f32; 2]; 3] {
    [t[0], t[1], t[2]].map(|i| f.vertices[i as usize])
}

fn area(f: &Fill) -> f32 {
    f.indices
        .chunks(3)
        .map(|t| {
            let [a, b, c] = tri(f, t);
            ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
        })
        .sum()
}

fn covers(f: &Fill, p: [f32; 2]) -> bool {
    f.indices.chunks(3).any(|t| {
        let [a, b, c] = tri(f, t);
        let s = |u: [f32; 2], v: [f32; 2]| (v[0] - u[0]) * (p[1] - u[1]) - (p[0] - u[0]) * (v[1] - u[1]);
        let (d1, d2, d3) = (s(a, b), s(b, c), s(c, a));
        !((d1 < 0.0 || d2 < 0.0 || d3 < 0.0) && (d1 > 0.0 || d2 > 0.0 || d3 > 0.0))
    })
}

fn miter_at(f: &Fill, p: [f32; 2]) -> [f32; 2] {
    f.contours
        .iter()
        .flatten()
        .find(|(q, _)| *q == p)
        .map(|(_, m)| *m)
        .unwrap_or_else(|| panic!("no contour point at {p:?}"))
}

fn close(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3
}

#[test]
fn square_with_hole_meshes_its_area() {
    let f = fill_of("square-hole");
    assert!((area(&f) - 220.0).abs() < 0.01, "area {}", area(&f));
    assert!(covers(&f, [6.0, 6.0]));
    assert!(!covers(&f, [12.0, 12.0]), "the hole stays empty");
    assert_eq!(f.contours.len(), 2);
}

#[test]
fn fill_rule_decides_the_star_centre() {
    let even = fill_of("star-evenodd");
    let nonzero = fill_of("star-nonzero");
    assert!(!covers(&even, [12.0, 12.0]), "even-odd leaves the centre empty");
    assert!(covers(&nonzero, [12.0, 12.0]), "non-zero fills the centre");
    assert!(covers(&even, [12.0, 5.0]) && covers(&nonzero, [12.0, 5.0]), "tips are filled under both");
}

#[test]
fn reversed_contour_is_a_hole_under_nonzero() {
    let f = fill_of("ring");
    assert!(covers(&f, [12.0, 4.0]));
    assert!(!covers(&f, [12.0, 12.0]));
}

#[test]
fn miters_point_away_from_the_fill() {
    let f = fill_of("square-hole");
    assert!(close(miter_at(&f, [4.0, 4.0]), [-1.0, -1.0]), "outer corner points out");
    assert!(close(miter_at(&f, [9.0, 9.0]), [1.0, 1.0]), "hole corner points into the hole");
    let f = fill_of("ring");
    assert!(close(miter_at(&f, [8.0, 8.0]), [1.0, 1.0]), "non-zero hole corner points into the hole");
}

#[test]
fn edges_with_fill_on_both_sides_get_no_fringe() {
    let f = fill_of("overlap");
    assert_eq!(miter_at(&f, [8.0, 8.0]), [0.0, 0.0], "both edges at (8, 8) lie inside the first square");
    assert!(close(miter_at(&f, [22.0, 22.0]), [1.0, 1.0]));
    assert!(close(miter_at(&f, [2.0, 2.0]), [-1.0, -1.0]));
}

#[test]
fn view_box_is_normalised_to_24_units() {
    let f = fill_of("big-box");
    let xs: Vec<f32> = f.vertices.iter().flat_map(|v| *v).collect();
    let (min, max) = xs.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    assert_eq!((min, max), (3.0, 21.0));
}

#[test]
fn strokes_and_fills_mix_in_one_icon() {
    let icon = convert("mixed", &fixture("mixed")).expect("converts");
    assert_eq!(icon.segs, vec![Seg::M(4.0, 12.0), Seg::L(20.0, 12.0)]);
    let f = icon.fill.expect("the dot is a fill");
    // Flattening at 0.02 units loses a little area against the true disc.
    assert!((area(&f) - std::f32::consts::PI * 4.0).abs() < 0.25, "disc area {}", area(&f));
}

#[test]
fn records_round_trip_through_quantisation() {
    for name in ["mixed", "ring", "big-box", "square-hole"] {
        let icon = convert(name, &fixture(name)).expect("converts");
        let (segs, fill) = decode(&encode(&icon).expect("encodes"));
        let scale = 24.0 / icon.size;
        assert_eq!(segs.len(), icon.segs.len(), "{name}");
        for (got, want) in segs.iter().zip(&icon.segs) {
            let (g, w) = (coords(*got), coords(*want));
            assert_eq!(g.len(), w.len(), "{name}");
            for (a, b) in g.iter().zip(w) {
                assert!((a - b * scale).abs() <= 0.5 / COORD_SCALE, "{name}: {got:?} vs {want:?}");
            }
        }
        let (got, want) = (fill.expect("fill survives"), icon.fill.expect("fill"));
        assert_eq!(got.vertices, want.vertices, "{name}: vertices are already quantised");
        assert_eq!(got.indices, want.indices, "{name}");
        for (gc, wc) in got.contours.iter().zip(&want.contours) {
            for ((gp, gm), (wp, wm)) in gc.iter().zip(wc) {
                assert_eq!(gp, wp, "{name}");
                assert!((gm[0] - wm[0]).abs() <= 0.5 / MITER_SCALE && (gm[1] - wm[1]).abs() <= 0.5 / MITER_SCALE, "{name}");
            }
        }
    }
}

fn coords(s: Seg) -> Vec<f32> {
    match s {
        Seg::M(x, y) | Seg::L(x, y) => vec![x, y],
        Seg::Q(a, b, x, y) => vec![a, b, x, y],
        Seg::C(a, b, c, d, x, y) => vec![a, b, c, d, x, y],
        Seg::Z => vec![],
    }
}

#[test]
fn stroke_only_icons_have_an_all_zero_fill_section() {
    let icon = convert("x", &fixture("x")).expect("converts");
    let rec = encode(&icon).expect("encodes");
    assert_eq!(&rec[rec.len() - 6..], &[0, 0, 0, 0, 0, 0]);
    assert_eq!(decode(&rec).1, None);
}

#[test]
fn coordinates_outside_the_range_are_errors() {
    let icon = convert("far", &fixture("far")).expect("converts");
    let err = encode(&icon).expect_err("40 units does not fit an i16 at 1/1024");
    assert!(err.contains("far") && err.contains("outside"), "{err}");
}

#[test]
fn crate_source_has_consts_all_and_slots() {
    let icons: Vec<_> = ["x", "ring", "mixed"].iter().map(|n| convert(n, &fixture(n)).expect("converts")).collect();
    let slots = parse_slots("check=mixed,chevron_down=ring,close=x").expect("parses");
    let (src, blob) = render_crate(&icons, &slots, "Test set 1.0 (MIT)").expect("renders");
    assert!(src.starts_with("// Generated by icongen"), "{src}");
    assert!(src.contains("//! Test set 1.0 (MIT)\n"), "{src}");
    assert!(src.contains("const BLOB: &[u8] = include_bytes!(\"icons.bin\");"), "{src}");
    // Sorted by name: mixed, ring, x.
    let mixed_len = encode(&icons[2]).expect("encodes").len();
    assert!(src.contains(&format!("pub const MIXED: Icon = icon(\"mixed\", 0, {mixed_len});")), "{src}");
    assert!(src.contains(&format!("pub const RING: Icon = icon(\"ring\", {mixed_len}, ")), "{src}");
    assert!(src.contains("pub const ALL: &[&Icon] = &[&MIXED, &RING, &X];"), "{src}");
    assert!(src.contains("pub const SLOTS: IconSlots = IconSlots { check: MIXED, chevron_down: RING, close: X };"), "{src}");
    let total: usize = icons.iter().map(|i| encode(i).expect("encodes").len()).sum();
    assert_eq!(blob.len(), total);
}

#[test]
fn slots_must_name_existing_icons() {
    assert!(parse_slots("check=a,chevron_down=b").is_err(), "close is missing");
    assert!(parse_slots("check=a,chevron_down=b,close=c,tick=d").is_err(), "unknown slot");
    let icons = vec![convert("x", &fixture("x")).expect("converts")];
    let slots = parse_slots("check=x,chevron_down=x,close=nope").expect("parses");
    let err = render_crate(&icons, &slots, "").expect_err("nope is not an icon");
    assert!(err.contains("close=nope"), "{err}");
}

#[test]
fn measure_reports_count_and_bytes() {
    let icons: Vec<_> = ["x", "ring"].iter().map(|n| convert(n, &fixture(n)).expect("converts")).collect();
    let report = measure(&icons).expect("measures");
    let total: usize = icons.iter().map(|i| encode(i).expect("encodes").len()).sum();
    assert!(report.starts_with("icons: 2\n"), "{report}");
    assert!(report.contains(&format!("blob bytes: {total} ")), "{report}");
    assert!(report.contains("largest: ring "), "the ring outweighs the cross: {report}");
}
```

In `tools/icongen/tests/convert.rs`, replace `fills_are_rejected` with:

```rust
#[test]
fn stroke_only_check_rejects_fills() {
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect width="4" height="4" fill="black"/></svg>"#;
    let icon = convert("filled", svg).expect("fills convert now");
    assert!(icon.fill.is_some());
    let err = icongen::ensure_stroke_only(&icon).expect_err("--out writes strokes only");
    assert!(err.contains("filled"), "{err}");
}
```

- [ ] **Step 3: Run them to see them fail**

Run: `cargo test --manifest-path tools/icongen/Cargo.toml --jobs 4 -- --test-threads 4`
Expected: FAIL to compile (`icongen::blob`, `measure`, `parse_slots`, `render_crate`, `Fill`, `ensure_stroke_only` not found).

- [ ] **Step 4: Add the dependency**

`tools/icongen/Cargo.toml`, `[dependencies]`:

```toml
# Tessellates filled paths at generation time; the runtime gets plain data.
lyon_tessellation = "1"
```

- [ ] **Step 5: Implement `fill.rs`**

`tools/icongen/src/fill.rs`:

```rust
//! Filled paths: flatten to polylines, quantise, tessellate with lyon and
//! compute outward miters for the painter's one-pixel fringe.

use crate::blob::quantise;
use lyon_tessellation::math::point;
use lyon_tessellation::path::iterator::PathIterator;
use lyon_tessellation::path::{Path, PathEvent};
use lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};
use usvg::tiny_skia_path::{PathSegment, Point};

/// Curve flattening tolerance in the 24-unit icon box.
pub const FLATTEN_TOLERANCE: f32 = 0.02;

/// Distance of the inside/outside probe from an edge's midpoint, in icon units.
const PROBE: f32 = 1e-3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

/// One filled SVG path as closed polylines in the 24-unit box, quantised.
#[derive(Clone, Debug, PartialEq)]
pub struct FillPath {
    pub rule: FillRule,
    pub contours: Vec<Vec<[f32; 2]>>,
}

/// The fill of one icon, as the blob stores it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fill {
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
    /// Per contour: (point, outward miter). A zero miter marks a vertex
    /// whose edges have fill on both sides; the painter's fringe vanishes there.
    pub contours: Vec<Vec<([f32; 2], [f32; 2])>>,
}

/// Flatten one usvg path (transform applied, then scaled into the 24-unit
/// box) into closed, quantised polylines. Consecutive duplicate points and a
/// closing point equal to the first are dropped; contours with fewer than
/// three points are dropped.
pub fn flatten(path: &usvg::tiny_skia_path::Path, ts: usvg::Transform, scale: f32) -> Vec<Vec<[f32; 2]>> {
    let map = |mut p: Point| {
        ts.map_point(&mut p);
        point(p.x * scale, p.y * scale)
    };
    let mut b = Path::builder();
    let mut open = false;
    let mut start = point(0.0, 0.0);
    for seg in path.segments() {
        // SVG continues from the subpath's start after Z.
        if !open && !matches!(seg, PathSegment::MoveTo(_) | PathSegment::Close) {
            b.begin(start);
            open = true;
        }
        match seg {
            PathSegment::MoveTo(p) => {
                if open {
                    b.end(true);
                }
                start = map(p);
                b.begin(start);
                open = true;
            }
            PathSegment::LineTo(p) => {
                b.line_to(map(p));
            }
            PathSegment::QuadTo(c, p) => {
                b.quadratic_bezier_to(map(c), map(p));
            }
            PathSegment::CubicTo(c1, c2, p) => {
                b.cubic_bezier_to(map(c1), map(c2), map(p));
            }
            PathSegment::Close => {
                if open {
                    b.end(true);
                    open = false;
                }
            }
        }
    }
    if open {
        b.end(true);
    }

    let mut contours = Vec::new();
    let mut cur: Vec<[f32; 2]> = Vec::new();
    for evt in b.build().iter().flattened(FLATTEN_TOLERANCE) {
        match evt {
            PathEvent::Begin { at } => {
                cur.clear();
                push(&mut cur, at.x, at.y);
            }
            PathEvent::Line { to, .. } => push(&mut cur, to.x, to.y),
            PathEvent::End { .. } => {
                if cur.len() > 1 && cur.first() == cur.last() {
                    cur.pop();
                }
                if cur.len() >= 3 {
                    contours.push(std::mem::take(&mut cur));
                }
                cur.clear();
            }
            PathEvent::Quadratic { .. } | PathEvent::Cubic { .. } => unreachable!("flattened paths hold only lines"),
        }
    }
    contours
}

fn push(c: &mut Vec<[f32; 2]>, x: f32, y: f32) {
    let q = [quantise(x), quantise(y)];
    if c.last() != Some(&q) {
        c.push(q);
    }
}

/// Tessellate every filled path of one icon and compute the contour miters.
/// `None` when the icon has no fill.
pub fn build(name: &str, paths: &[FillPath]) -> Result<Option<Fill>, String> {
    let mut fill = Fill::default();
    for fp in paths {
        let mut b = Path::builder();
        for c in &fp.contours {
            b.begin(point(c[0][0], c[0][1]));
            for p in &c[1..] {
                b.line_to(point(p[0], p[1]));
            }
            b.end(true);
        }
        let rule = match fp.rule {
            FillRule::NonZero => lyon_tessellation::FillRule::NonZero,
            FillRule::EvenOdd => lyon_tessellation::FillRule::EvenOdd,
        };
        let mut buf: VertexBuffers<[f32; 2], u16> = VertexBuffers::new();
        FillTessellator::new()
            .tessellate_path(
                &b.build(),
                &FillOptions::tolerance(FLATTEN_TOLERANCE).with_fill_rule(rule),
                &mut BuffersBuilder::new(&mut buf, |v: FillVertex| v.position().to_array()),
            )
            .map_err(|e| format!("{name}: tessellation failed: {e:?}"))?;
        let base = fill.vertices.len();
        if base + buf.vertices.len() > usize::from(u16::MAX) {
            return Err(format!("{name}: fill mesh has more than 65535 vertices"));
        }
        // Boundary vertices are already quantised; intersection vertices lyon
        // adds are quantised here so the blob stores exactly these values.
        fill.vertices.extend(buf.vertices.iter().map(|v| [quantise(v[0]), quantise(v[1])]));
        fill.indices.extend(buf.indices.iter().map(|&i| i + base as u16));
        for c in &fp.contours {
            fill.contours.push(miters(c, fp, paths));
        }
    }
    Ok((!fill.vertices.is_empty()).then_some(fill))
}

/// Outward miter per vertex of the closed contour `c` of `own`.
fn miters(c: &[[f32; 2]], own: &FillPath, all: &[FillPath]) -> Vec<([f32; 2], [f32; 2])> {
    let n = c.len();
    // Outward unit normal of edge i (c[i] -> c[i+1]); zero when both sides are filled.
    let normals: Vec<[f32; 2]> = (0..n)
        .map(|i| {
            let (a, b) = (c[i], c[(i + 1) % n]);
            let d = normalize([b[0] - a[0], b[1] - a[1]]);
            let left = [d[1], -d[0]];
            let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            let probe = |v: [f32; 2]| [mid[0] + v[0] * PROBE, mid[1] + v[1] * PROBE];
            let out = if inside(own, probe(left)) { [-left[0], -left[1]] } else { left };
            if all.iter().any(|fp| inside(fp, probe(out))) {
                [0.0, 0.0]
            } else {
                out
            }
        })
        .collect();
    (0..n).map(|i| (c[i], miter(normals[(i + n - 1) % n], normals[i]))).collect()
}

/// The miter at a vertex between edges with outward normals `n1` (incoming)
/// and `n2` (outgoing): `normalize(n1 + n2) / max(dot(m, n1), 0.5)`, so its
/// length is at most 2. A zero normal (edge with fill on both sides) yields
/// the other edge's normal; two zero normals yield zero.
pub fn miter(n1: [f32; 2], n2: [f32; 2]) -> [f32; 2] {
    let zero = [0.0, 0.0];
    match (n1 == zero, n2 == zero) {
        (true, true) => zero,
        (true, false) => n2,
        (false, true) => n1,
        (false, false) => {
            let s = [n1[0] + n2[0], n1[1] + n2[1]];
            let len = (s[0] * s[0] + s[1] * s[1]).sqrt();
            if len < 1e-6 {
                return n1;
            }
            let m = [s[0] / len, s[1] / len];
            let k = 1.0 / (m[0] * n1[0] + m[1] * n1[1]).max(0.5);
            [m[0] * k, m[1] * k]
        }
    }
}

fn normalize(v: [f32; 2]) -> [f32; 2] {
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    [v[0] / len, v[1] / len]
}

/// Whether `p` lies inside the fill of `fp` under its fill rule.
pub fn inside(fp: &FillPath, p: [f32; 2]) -> bool {
    let w = winding(&fp.contours, p);
    match fp.rule {
        FillRule::NonZero => w != 0,
        FillRule::EvenOdd => w % 2 != 0,
    }
}

fn winding(contours: &[Vec<[f32; 2]>], p: [f32; 2]) -> i32 {
    let mut w = 0;
    for c in contours {
        for i in 0..c.len() {
            let (a, b) = (c[i], c[(i + 1) % c.len()]);
            let cross = (b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1]);
            if a[1] <= p[1] {
                if b[1] > p[1] && cross > 0.0 {
                    w += 1;
                }
            } else if b[1] <= p[1] && cross < 0.0 {
                w -= 1;
            }
        }
    }
    w
}
```

(lyon_tessellation 1.x: `Path::builder()` with `begin/line_to/quadratic_bezier_to/cubic_bezier_to/end(close)`,
`PathIterator::flattened`, `FillOptions::tolerance(..).with_fill_rule(..)`. If a name
differs in the resolved version, check `cargo doc --manifest-path tools/icongen/Cargo.toml --open`
and keep the behaviour described in the doc comments.)

- [ ] **Step 6: Implement `blob.rs`**

`tools/icongen/src/blob.rs`:

```rust
//! The icon blob record, read by `shadcn_egui_core`'s decoder. All values
//! are little-endian 16-bit words.
//!
//! - Stroke section: `len` (words that follow), then opcodes, each followed by
//!   its coordinates: 0=M(2) 1=L(2) 2=Q(4) 3=C(6) 4=Z(0). Coordinates are in
//!   the 24-unit box, stored as `round(v * 1024)` (i16, so −32..32 units).
//! - Fill section (always present; all zero = no fill): `nv, ni, nc`;
//!   `nv × (x, y)`; `ni × index` (u16); `nc ×` contours `{ len, len × (x, y, mx, my) }`,
//!   miters stored as `round(m * 8192)`, |m| ≤ 2.

use crate::{Converted, Fill, Seg};

pub const COORD_SCALE: f32 = 1024.0;
pub const MITER_SCALE: f32 = 8192.0;

/// `v` rounded to the stored precision; the decoder returns exactly this value.
pub fn quantise(v: f32) -> f32 {
    (v * COORD_SCALE).round() / COORD_SCALE
}

fn coord(name: &str, v: f32) -> Result<u16, String> {
    let q = (v * COORD_SCALE).round();
    if !(f32::from(i16::MIN)..=f32::from(i16::MAX)).contains(&q) {
        return Err(format!("{name}: coordinate {v} outside -32..32 icon units"));
    }
    Ok(q as i16 as u16)
}

fn miter(m: f32) -> u16 {
    (m.clamp(-2.0, 2.0) * MITER_SCALE).round() as i16 as u16
}

fn count(name: &str, what: &str, n: usize) -> Result<u16, String> {
    u16::try_from(n).map_err(|_| format!("{name}: {n} {what}, at most 65535"))
}

/// Encode one icon. Stroke coordinates are scaled from the SVG's units into
/// the 24-unit box; the fill is already there.
pub fn encode(icon: &Converted) -> Result<Vec<u8>, String> {
    let name = icon.name.as_str();
    let scale = 24.0 / icon.size;
    let mut stroke: Vec<u16> = Vec::new();
    for seg in &icon.segs {
        let (op, coords) = match *seg {
            Seg::M(x, y) => (0, vec![x, y]),
            Seg::L(x, y) => (1, vec![x, y]),
            Seg::Q(a, b, x, y) => (2, vec![a, b, x, y]),
            Seg::C(a, b, c, d, x, y) => (3, vec![a, b, c, d, x, y]),
            Seg::Z => (4, vec![]),
        };
        stroke.push(op);
        for v in coords {
            stroke.push(coord(name, v * scale)?);
        }
    }
    let mut w = Vec::with_capacity(stroke.len() + 4);
    w.push(count(name, "stroke words", stroke.len())?);
    w.extend(stroke);
    match &icon.fill {
        None => w.extend([0, 0, 0]),
        Some(f) => {
            w.push(count(name, "fill vertices", f.vertices.len())?);
            w.push(count(name, "fill indices", f.indices.len())?);
            w.push(count(name, "contours", f.contours.len())?);
            for v in &f.vertices {
                w.push(coord(name, v[0])?);
                w.push(coord(name, v[1])?);
            }
            w.extend(&f.indices);
            for c in &f.contours {
                w.push(count(name, "contour points", c.len())?);
                for (p, m) in c {
                    w.push(coord(name, p[0])?);
                    w.push(coord(name, p[1])?);
                    w.push(miter(m[0]));
                    w.push(miter(m[1]));
                }
            }
        }
    }
    Ok(w.iter().flat_map(|v| v.to_le_bytes()).collect())
}

/// Bytes of a record's stroke section, including its length word.
pub fn stroke_bytes(record: &[u8]) -> usize {
    2 + 2 * usize::from(u16::from_le_bytes([record[0], record[1]]))
}

/// Reference decoder for tests: one record back into strokes and fill.
pub fn decode(bytes: &[u8]) -> (Vec<Seg>, Option<Fill>) {
    let mut words = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]));
    let mut next = || words.next().expect("truncated record");
    let c = |w: u16| f32::from(w as i16) / COORD_SCALE;
    let m = |w: u16| f32::from(w as i16) / MITER_SCALE;
    let stroke_len = usize::from(next());
    let (mut segs, mut used) = (Vec::new(), 0);
    while used < stroke_len {
        let op = next();
        let n = [2, 2, 4, 6, 0][usize::from(op)];
        let v: Vec<f32> = (0..n).map(|_| c(next())).collect();
        used += 1 + n;
        segs.push(match op {
            0 => Seg::M(v[0], v[1]),
            1 => Seg::L(v[0], v[1]),
            2 => Seg::Q(v[0], v[1], v[2], v[3]),
            3 => Seg::C(v[0], v[1], v[2], v[3], v[4], v[5]),
            _ => Seg::Z,
        });
    }
    let (nv, ni, nc) = (usize::from(next()), usize::from(next()), usize::from(next()));
    if nv == 0 && nc == 0 {
        return (segs, None);
    }
    let vertices = (0..nv).map(|_| [c(next()), c(next())]).collect();
    let indices = (0..ni).map(|_| next()).collect();
    let contours = (0..nc)
        .map(|_| {
            let len = usize::from(next());
            (0..len).map(|_| ([c(next()), c(next())], [m(next()), m(next())])).collect()
        })
        .collect();
    (segs, Some(Fill { vertices, indices, contours }))
}
```

- [ ] **Step 7: Rework `lib.rs`**

In `tools/icongen/src/lib.rs`:

1. Module doc: replace the paragraph starting "The input is an outline icon set" with:

```rust
//! The input is an icon set such as Lucide (stroked outlines) or Phosphor
//! (filled outlines). usvg resolves shapes, arcs and transforms into
//! move/line/quad/cubic segments. Stroked paths become `Seg` values; filled
//! paths are flattened and triangulated (see `fill`). `--out` writes Rust
//! `Seg` slices (strokes only); `--crate-dir` writes a whole icon crate as a
//! binary blob (see `blob`). Images, text and non-square view boxes are
//! errors, not silent omissions.
```

2. Below the `use` line add:

```rust
pub mod blob;
pub mod fill;

pub use fill::{Fill, FillPath, FillRule};
```

3. Replace `Converted`, `convert` and `walk` with:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Converted {
    /// The icon's name as given (usually the SVG file stem), e.g. `circle-alert`.
    pub name: String,
    /// Side of the square view box; `segs` are in these units.
    pub size: f32,
    /// Stroked subpaths.
    pub segs: Vec<Seg>,
    /// Filled paths, tessellated, in the 24-unit box and quantised.
    pub fill: Option<Fill>,
}

/// Convert one SVG document. `name` becomes the icon's accessible name and,
/// upper-cased, its constant name.
pub fn convert(name: &str, svg: &str) -> Result<Converted, String> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| format!("{name}: {e}"))?;
    let size = tree.size();
    if size.width() != size.height() {
        return Err(format!("{name}: view box is {}x{}, icons must be square", size.width(), size.height()));
    }
    let (mut segs, mut fills) = (Vec::new(), Vec::new());
    walk(name, tree.root(), 24.0 / size.width(), &mut segs, &mut fills)?;
    let fill = fill::build(name, &fills)?;
    if segs.is_empty() && fill.is_none() {
        return Err(format!("{name}: no stroked or filled paths"));
    }
    Ok(Converted { name: name.to_owned(), size: size.width(), segs, fill })
}

fn walk(name: &str, group: &usvg::Group, scale: f32, segs: &mut Vec<Seg>, fills: &mut Vec<FillPath>) -> Result<(), String> {
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => walk(name, g, scale, segs, fills)?,
            usvg::Node::Path(p) => {
                if !p.is_visible() {
                    continue;
                }
                if let Some(f) = p.fill() {
                    let rule = match f.rule() {
                        usvg::FillRule::NonZero => FillRule::NonZero,
                        usvg::FillRule::EvenOdd => FillRule::EvenOdd,
                    };
                    let contours = fill::flatten(p.data(), p.abs_transform(), scale);
                    if !contours.is_empty() {
                        fills.push(FillPath { rule, contours });
                    }
                }
                if p.stroke().is_some() {
                    path_segs(p.data(), p.abs_transform(), segs);
                }
            }
            usvg::Node::Image(_) => return Err(format!("{name}: embedded image not supported")),
            usvg::Node::Text(_) => return Err(format!("{name}: text not supported")),
        }
    }
    Ok(())
}

/// `--out` writes `Seg` slices, which cannot hold a fill.
pub fn ensure_stroke_only(icon: &Converted) -> Result<(), String> {
    match icon.fill {
        Some(_) => Err(format!("{}: filled shape; --out writes strokes only, use --crate-dir", icon.name)),
        None => Ok(()),
    }
}
```

4. Append the crate renderer and the measurement:

```rust
/// The three icons shadcn-egui's components draw, by icon name.
#[derive(Clone, Debug, PartialEq)]
pub struct Slots {
    pub check: String,
    pub chevron_down: String,
    pub close: String,
}

/// Parse `check=<icon>,chevron_down=<icon>,close=<icon>`.
pub fn parse_slots(s: &str) -> Result<Slots, String> {
    let (mut check, mut chevron_down, mut close) = (None, None, None);
    for part in s.split(',') {
        let (k, v) = part.split_once('=').ok_or_else(|| format!("--slots: {part:?} is not slot=icon"))?;
        let slot = match k {
            "check" => &mut check,
            "chevron_down" => &mut chevron_down,
            "close" => &mut close,
            _ => return Err(format!("--slots: unknown slot {k:?}")),
        };
        *slot = Some(v.to_owned());
    }
    match (check, chevron_down, close) {
        (Some(check), Some(chevron_down), Some(close)) => Ok(Slots { check, chevron_down, close }),
        _ => Err("--slots needs check=<icon>,chevron_down=<icon>,close=<icon>".to_owned()),
    }
}

/// Render an icon crate: `src/lib.rs` text and the `src/icons.bin` bytes.
/// Icons are sorted by name, so the output does not depend on argument order.
pub fn render_crate(icons: &[Converted], slots: &Slots, provenance: &str) -> Result<(String, Vec<u8>), String> {
    let mut sorted: Vec<&Converted> = icons.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    let find = |slot: &str, name: &str| {
        sorted
            .iter()
            .find(|i| i.name == name)
            .map(|i| const_name(&i.name))
            .ok_or_else(|| format!("--slots: {slot}={name}: no such icon"))
    };
    let check = find("check", &slots.check)?;
    let chevron_down = find("chevron_down", &slots.chevron_down)?;
    let close = find("close", &slots.close)?;

    let mut src = String::new();
    src.push_str("// Generated by icongen (shadcn-egui/tools/icongen). Do not edit: re-run it.\n");
    for line in provenance.lines() {
        src.push_str(&format!("//! {line}\n"));
    }
    src.push_str("//!\n//! Every icon is a `pub const`; [`ALL`] lists them and [`SLOTS`] holds the\n");
    src.push_str("//! icons shadcn-egui's components draw (checkbox tick, select trigger, close button).\n\n");
    src.push_str("use shadcn_egui_core::{BlobRange, Icon, IconShape, IconSlots};\n\n");
    src.push_str("const BLOB: &[u8] = include_bytes!(\"icons.bin\");\n\n");
    src.push_str("const fn icon(name: &'static str, start: u32, len: u32) -> Icon {\n");
    src.push_str("    Icon { name, size: 24.0, shape: IconShape::Blob(BlobRange { blob: BLOB, start, len }) }\n}\n\n");
    let mut blob = Vec::new();
    for icon in &sorted {
        let rec = blob::encode(icon)?;
        src.push_str(&format!(
            "pub const {}: Icon = icon({:?}, {}, {});\n",
            const_name(&icon.name),
            icon.name,
            blob.len(),
            rec.len()
        ));
        blob.extend_from_slice(&rec);
    }
    if u32::try_from(blob.len()).is_err() {
        return Err(format!("blob is {} bytes, more than u32 offsets reach", blob.len()));
    }
    src.push_str("\n/// Every icon in this crate, sorted by name.\npub const ALL: &[&Icon] = &[");
    src.push_str(&sorted.iter().map(|i| format!("&{}", const_name(&i.name))).collect::<Vec<_>>().join(", "));
    src.push_str("];\n\n/// The icons shadcn-egui's components draw.\n");
    src.push_str(&format!(
        "pub const SLOTS: IconSlots = IconSlots {{ check: {check}, chevron_down: {chevron_down}, close: {close} }};\n"
    ));
    Ok((src, blob))
}

/// Icon count, blob size split into stroke and fill, and the ten largest icons.
pub fn measure(icons: &[Converted]) -> Result<String, String> {
    let (mut stroke, mut fill, mut sizes) = (0, 0, Vec::new());
    for icon in icons {
        let rec = blob::encode(icon)?;
        let s = blob::stroke_bytes(&rec);
        stroke += s;
        fill += rec.len() - s;
        sizes.push((rec.len(), icon.name.as_str()));
    }
    sizes.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    let largest: Vec<String> = sizes.iter().take(10).map(|(n, name)| format!("{name} {n}")).collect();
    Ok(format!(
        "icons: {}\nblob bytes: {} (stroke {stroke}, fill {fill})\nlargest: {}\n",
        icons.len(),
        stroke + fill,
        largest.join(", ")
    ))
}
```

- [ ] **Step 8: Rework the CLI**

Replace `tools/icongen/src/main.rs` with:

```rust
//! icongen: SVG icons -> shadcn-egui icon data.
//!
//! icongen --out src/icons.rs [--module-path crate::components::icon]
//!         [--provenance "Lucide v1.54.0 (ISC)"] path/to/mic.svg ...
//!     Rust `Seg` slices, strokes only (filled shapes are an error).
//! icongen --crate-dir crates/shadcn-egui-icons-x --slots check=..,chevron_down=..,close=..
//!         [--provenance TEXT] [--measure] path/to/*.svg
//!     A whole icon crate: DIR/src/lib.rs and DIR/src/icons.bin.
//! icongen --measure path/to/*.svg
//!     Print icon count, blob bytes and the largest icons; write nothing.
//!
//! Each icon is named after its file stem.

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: icongen --out FILE [--module-path PATH] [--provenance TEXT] SVG...\n       icongen --crate-dir DIR --slots check=I,chevron_down=I,close=I [--provenance TEXT] [--measure] SVG...\n       icongen --measure SVG...";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("icongen: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let (mut out, mut crate_dir, mut slots, mut measure) = (None, None, None, false);
    let mut module_path = "crate::components::icon".to_owned();
    let mut provenance = String::new();
    let mut inputs = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut value = |flag: &str| it.next().ok_or_else(|| format!("{flag} needs a value\n{USAGE}"));
        match a.as_str() {
            "--out" => out = Some(value("--out")?),
            "--crate-dir" => crate_dir = Some(value("--crate-dir")?),
            "--slots" => slots = Some(icongen::parse_slots(&value("--slots")?)?),
            "--measure" => measure = true,
            "--module-path" => module_path = value("--module-path")?,
            "--provenance" => provenance = value("--provenance")?,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            f if f.starts_with("--") => return Err(format!("unknown flag {f}\n{USAGE}")),
            _ => inputs.push(a),
        }
    }
    if out.is_some() && crate_dir.is_some() {
        return Err(format!("--out and --crate-dir exclude each other\n{USAGE}"));
    }
    if out.is_none() && crate_dir.is_none() && !measure {
        return Err(format!("one of --out, --crate-dir, --measure is required\n{USAGE}"));
    }
    if inputs.is_empty() {
        return Err(format!("no SVG files given\n{USAGE}"));
    }

    let mut icons = Vec::new();
    let mut seen = std::collections::HashMap::new();
    for input in &inputs {
        let path = Path::new(input);
        let name = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| format!("{input}: no file name"))?;
        let svg = std::fs::read_to_string(path).map_err(|e| format!("{input}: {e}"))?;
        let icon = icongen::convert(name, &svg)?;
        if let Some(prev) = seen.insert(icongen::const_name(name), input.clone()) {
            return Err(format!("{input} and {prev} both map to constant {}", icongen::const_name(name)));
        }
        icons.push(icon);
    }
    if measure {
        print!("{}", icongen::measure(&icons)?);
    }
    if let Some(out) = out {
        for icon in &icons {
            icongen::ensure_stroke_only(icon)?;
        }
        std::fs::write(&out, icongen::render_module(&icons, &module_path, &provenance))
            .map_err(|e| format!("{out}: {e}"))?;
        eprintln!("icongen: wrote {} icons to {out}", icons.len());
    }
    if let Some(dir) = crate_dir {
        let slots = slots.ok_or_else(|| format!("--crate-dir needs --slots\n{USAGE}"))?;
        let (src, blob) = icongen::render_crate(&icons, &slots, &provenance)?;
        let src_dir = Path::new(&dir).join("src");
        std::fs::create_dir_all(&src_dir).map_err(|e| format!("{}: {e}", src_dir.display()))?;
        std::fs::write(src_dir.join("lib.rs"), src).map_err(|e| format!("{dir}/src/lib.rs: {e}"))?;
        std::fs::write(src_dir.join("icons.bin"), &blob).map_err(|e| format!("{dir}/src/icons.bin: {e}"))?;
        eprintln!("icongen: wrote {} icons ({} blob bytes) to {dir}", icons.len(), blob.len());
    }
    Ok(())
}
```

- [ ] **Step 9: Run the tests**

Run: `cargo test --manifest-path tools/icongen/Cargo.toml --jobs 4 -- --test-threads 4`
Expected: PASS, all tests in `convert.rs` and `fill.rs`. If
`miters_point_away_from_the_fill` fails for the ring, print `f.contours` and
check the probe side before touching the tolerance.

Run: `cargo clippy --manifest-path tools/icongen/Cargo.toml --all-targets --jobs 4 -- -D warnings`
Expected: no warnings.

Run (the `--out` fixture must not change in this task):

```bash
cargo run --manifest-path tools/icongen/Cargo.toml --jobs 4 -- \
  --out crates/shadcn-egui/tests/icons/lucide.rs \
  --module-path shadcn_egui::components::icon \
  --provenance "Lucide v1.54.0 (ISC), test fixtures from tools/icongen/tests/fixtures" \
  tools/icongen/tests/fixtures/{mic,circle-alert,settings,x,power,monitor}.svg
git diff --exit-code crates/shadcn-egui/tests/icons/lucide.rs
```

Expected: exit 0.

- [ ] **Step 10: Commit**

```bash
git add tools/icongen
git commit -m "feat(icongen): filled paths, binary blob records, --crate-dir and --measure

Filled SVG paths are flattened at 0.02 units, quantised to 1/1024 and
triangulated with lyon under their fill rule. Each contour vertex gets an
outward miter (probe of the winding number beside each edge; zero where
fill lies on both sides) for the painter's outward-only fringe. --crate-dir
writes a whole icon set as lib.rs plus one icons.bin blob; --measure
reports sizes so the format can be checked against crates.io limits.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6.2: Measure the full Remixicon and Phosphor sets (before fixing the format)

**Files:**
- Create: `docs/superpowers/notes/0.4/p6-icon-sizes.md`
- Modify (only in branch a or b below): `tools/icongen/src/fill.rs` or `tools/icongen/src/blob.rs`

**Interfaces:**
- Consumes: `tools/icongen/SETS.tsv` (`lib package version svg_glob license ship`), icongen `--measure` and `--crate-dir` from Task 6.1.
- Produces: `p6-icon-sizes.md` with, per set, icon count, blob bytes (stroke/fill), the ten largest icons, and the `.crate` size; the chosen branch (none, a, b). Task 6.3 reads the branch.

- [ ] **Step 1: Download the pinned packages**

```bash
fetch() { # fetch <npm name> <version> -> prints the extraction dir
  local dir="$SCRATCH/npm/$(echo "$1" | tr / _)-$2"
  if [ ! -d "$dir/package" ]; then
    mkdir -p "$dir"
    local tgz; tgz=$(curl -sf "https://registry.npmjs.org/$1/$2" | jq -r .dist.tarball)
    curl -sfL "$tgz" | tar -xz -C "$dir"
  fi
  echo "$dir"
}
row() { awk -F'\t' -v l="$1" '$1 == l' tools/icongen/SETS.tsv; }
for lib in remixicon phosphor tabler; do
  IFS=$'\t' read -r _ package version glob license ship < <(row "$lib")
  echo "$lib $package $version $glob $license $ship $(fetch "$package" "$version")"
done
```

Keep this shell open: Steps 2–3 use `fetch` and `row`.
Expected: three lines, each ending in an existing directory under
`$SCRATCH/npm/`. (curl sends its default User-Agent; nothing personal.)
Tabler is measured too because it is the largest set by count.

- [ ] **Step 2: Measure the blobs**

```bash
mkdir -p "$SCRATCH/measure"
cargo build --release --manifest-path tools/icongen/Cargo.toml --jobs 4
ICONGEN=$(cargo metadata --manifest-path tools/icongen/Cargo.toml --format-version 1 --no-deps | jq -r .target_directory)/release/icongen
for lib in remixicon phosphor tabler; do
  IFS=$'\t' read -r _ package version glob license ship < <(row "$lib")
  dir=$(fetch "$package" "$version")
  # shellcheck disable=SC2206  # the glob is meant to expand
  files=("$dir"/$glob)
  case $lib in
    remixicon) slots=check=check-line,chevron_down=arrow-down-s-line,close=close-line ;;
    phosphor)  slots=check=check,chevron_down=caret-down,close=x ;;
    tabler)    slots=check=check,chevron_down=selector,close=x ;;
  esac
  m="$SCRATCH/measure/$lib"
  "$ICONGEN" --measure --crate-dir "$m" --slots "$slots" --provenance "measurement" "${files[@]}" | tee "$SCRATCH/measure/$lib.txt"
done
```

Expected: per set, `icons: N`, `blob bytes: …`, `largest: …`. An icongen
error naming an icon (unsupported element, constant collision, coordinate
out of range) is a finding: record it and **STOP** to ask the user; do not
skip icons silently.

- [ ] **Step 3: Measure the packaged size**

```bash
for lib in remixicon phosphor tabler; do
  m="$SCRATCH/measure/$lib"
  cat > "$m/Cargo.toml" <<EOF
[package]
name = "measure-$lib"
version = "0.0.0"
edition = "2021"
license = "MIT"
description = "icon blob size measurement"

[workspace]
EOF
  cargo package --manifest-path "$m/Cargo.toml" --no-verify --allow-dirty \
    --target-dir "$SCRATCH/measure/target" --jobs 4
  ls -l "$SCRATCH/measure/target/package/measure-$lib-0.0.0.crate"
done
```

Expected: one `.crate` per set; note its byte size. (`--no-verify` packages
without compiling, so the generated `lib.rs` need not build yet.)

- [ ] **Step 4: Decide**

Branches (index D), in order, re-measuring (Steps 2–3) after each:

- Every `.crate` ≤ 8 MB → **no change**.
- Remixicon or Phosphor above 8 MB → **branch (a)**: in `tools/icongen/src/fill.rs`
  set `pub const FLATTEN_TOLERANCE: f32 = 0.05;`, re-run the icongen tests
  (`cargo test --manifest-path tools/icongen/Cargo.toml --jobs 4 -- --test-threads 4`;
  `strokes_and_fills_mix_in_one_icon` may need its disc-area tolerance raised
  from 0.25 to 0.5, nothing else), re-measure.
- Still above 8 MB → **branch (b)**: byte-delta indices. In `blob::encode`,
  replace `w.push(count(name, "fill indices", f.indices.len())?);` … `w.extend(&f.indices);`
  by the code below, and in `blob::decode` read the flag the same way:

  ```rust
  // Branch (b): when every delta to the previous index (the first to 0)
  // fits i8, store `ni | 0x8000` and one byte per index (delta + 128),
  // padded to a whole word. Otherwise u16 indices as before.
  let deltas: Option<Vec<u8>> = {
      let mut prev = 0i32;
      f.indices
          .iter()
          .map(|&i| {
              let d = i32::from(i) - prev;
              prev = i32::from(i);
              u8::try_from(d + 128).ok()
          })
          .collect()
  };
  let ni = count(name, "fill indices", f.indices.len())?;
  if ni >= 0x8000 {
      return Err(format!("{name}: {ni} fill indices, at most 32767"));
  }
  // (write nv before this block, nc after it, unchanged)
  ```

  …with the header words then written as `nv, ni | 0x8000 (if deltas is Some), nc`,
  vertices unchanged, and in place of the u16 indices the delta bytes
  (`deltas.chunks(2).map(|p| u16::from_le_bytes([p[0], *p.get(1).unwrap_or(&0)]))`).
  Add a round-trip assertion to `records_round_trip_through_quantisation`
  (it already compares `indices`) and re-measure.
- Still above 8 MB → **branch (c): STOP.** Report the numbers to the user and
  ask before splitting a crate.

- [ ] **Step 5: Record**

`docs/superpowers/notes/0.4/p6-icon-sizes.md`:

```markdown
# P6: icon set sizes (measured before fixing the blob format)

Pinned versions from `tools/icongen/SETS.tsv`; icongen at commit <short hash of Task 6.1>.

| Set | Version | Icons | Blob bytes (stroke / fill) | .crate bytes | Largest icons |
|---|---|---|---|---|---|
| remixicon | … | … | … | … | … |
| phosphor | … | … | … | … | … |
| tabler | … | … | … | … | … |

Branch taken: none | (a) tolerance 0.05 | (b) byte-delta indices.
Re-measured after the branch: <numbers, or "n/a">.
```

Fill every cell with the measured value from `$SCRATCH/measure/<lib>.txt`
and Step 3. Do not leave `…` in the committed file.

- [ ] **Step 6: Commit**

```bash
git add docs/superpowers/notes/0.4/p6-icon-sizes.md tools/icongen
git commit -m "docs: P6 measured icon blob sizes for Remixicon, Phosphor and Tabler

Measured with the full pinned sets before the format is fixed in the core
decoder; records the .crate sizes against the 8 MB gate and the branch taken.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6.3: Core `IconShape`, `BlobRange`, `SegIter`, `FillMesh` decoders

**Files:**
- Create: `crates/shadcn-egui-core/src/blob.rs`
- Modify: `crates/shadcn-egui-core/src/icon.rs` (`Icon.segs` → `Icon.shape`, `IconShape`, `SegIter`, `Icon::segs()`, `Icon::fill_mesh()`)
- Modify: `crates/shadcn-egui-core/src/lib.rs`
- Modify: `crates/shadcn-egui/src/components/icon.rs` (re-export `IconShape`; `paint_icon` iterates `icon.segs()`; the Phase 4 consts `CHECK`/`CHEVRON_DOWN`/`CLOSE` use `shape: IconShape::Segs(..)`)
- Modify: `tools/icongen/src/lib.rs` (`render_module` emits `shape: IconShape::Segs(&[..])`), `tools/icongen/tests/convert.rs` (`names_and_module_render`)
- Create: `tools/icongen/tests/fixture_crate.rs`
- Regenerate: `crates/shadcn-egui/tests/icons/lucide.rs`, `crates/shadcn-egui/tests/icons/fixture/src/{lib.rs,icons.bin}`
- Create: `crates/shadcn-egui/tests/icon_fill.rs`

**Interfaces:**
- Consumes: the record format of Task 6.1 (with the branch of Task 6.2), Phase 4's `IconSlots` in core and `LUCIDE_SLOTS`/`CHECK`/`CHEVRON_DOWN`/`CLOSE` in `components/icon.rs`.
- Produces (all `pub` in `shadcn_egui_core`, re-exported at the `shadcn_egui` root):
  - `struct Icon { pub name: &'static str, pub size: f32, pub shape: IconShape }` (`Clone, Copy, Debug, PartialEq`)
  - `enum IconShape { Segs(&'static [Seg]), Blob(BlobRange) }` (`Clone, Copy, Debug, PartialEq`)
  - `struct BlobRange { pub blob: &'static [u8], pub start: u32, pub len: u32 }` — `Clone, Copy`; `PartialEq` by blob pointer, start, len; `Debug` prints `BlobRange { start: …, len: … }`.
  - `Icon::segs(&self) -> SegIter`; `enum SegIter { Slice(core::slice::Iter<'static, Seg>), Words(BlobSegs) }: Iterator<Item = Seg>` (`Clone, Debug`).
  - **NEW INTERFACE:** `pub struct BlobSegs` (opaque iterator over a record's stroke words; `Clone`, `Debug` without bytes) — the payload type of `SegIter::Words`, which the index leaves as `Words(..)`.
  - `Icon::fill_mesh(&self) -> Option<FillMesh>`; `struct FillMesh { pub vertices: Vec<[f32; 2]>, pub indices: Vec<u16>, pub contours: Vec<Vec<([f32; 2], [f32; 2])>> }` (`Clone, Debug, Default, PartialEq`).
  - `shadcn_egui::components::icon::IconShape` (re-export, for `--out` modules).
  - Test fixture crate source `crates/shadcn-egui/tests/icons/fixture/src/lib.rs` with consts `BIG_BOX, MIXED, RING, SQUARE_HOLE, STAR_EVENODD, X`, `ALL`, `SLOTS` (Task 6.4 uses it).

- [ ] **Step 1: Write the failing core tests**

`crates/shadcn-egui-core/src/blob.rs` (tests at the bottom; the whole file
in Step 3). Add `mod blob;` to `crates/shadcn-egui-core/src/lib.rs` first so
the tests are compiled.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::icon::{Icon, IconShape, Seg};

    /// Words (as i16 values, so miters can be negative) to leaked LE bytes.
    fn leak(words: &[i32]) -> &'static [u8] {
        let bytes: Vec<u8> = words.iter().flat_map(|&w| (w as i16 as u16).to_le_bytes()).collect();
        Box::leak(bytes.into_boxed_slice())
    }

    /// M(1, 2) L(3, 4) Z, then a one-triangle fill with one contour.
    fn record() -> &'static [u8] {
        leak(&[
            7, 0, 1024, 2048, 1, 3072, 4096, 4, // stroke
            3, 3, 1, // nv ni nc
            0, 0, 1024, 0, 0, 1024, // vertices
            0, 1, 2, // indices
            3, 0, 0, -8192, -8192, 1024, 0, 8192, -8192, 0, 1024, -8192, 8192, // contour
        ])
    }

    fn blob_icon(blob: &'static [u8], start: u32, len: u32) -> Icon {
        Icon { name: "t", size: 24.0, shape: IconShape::Blob(BlobRange { blob, start, len }) }
    }

    #[test]
    fn blob_record_decodes_strokes_and_fill() {
        let blob = record();
        let icon = blob_icon(blob, 0, blob.len() as u32);
        assert_eq!(icon.segs().collect::<Vec<_>>(), vec![Seg::M(1.0, 2.0), Seg::L(3.0, 4.0), Seg::Z]);
        let fill = icon.fill_mesh().expect("has a fill");
        assert_eq!(fill.vertices, vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
        assert_eq!(fill.indices, vec![0, 1, 2]);
        assert_eq!(fill.contours.len(), 1);
        assert_eq!(fill.contours[0][0], ([0.0, 0.0], [-1.0, -1.0]));
        assert_eq!(fill.contours[0][1], ([1.0, 0.0], [1.0, -1.0]));
    }

    #[test]
    fn records_are_read_at_their_offset() {
        // M(1, 1) with no fill (7 words = 14 bytes), then Z with no fill.
        let blob = leak(&[3, 0, 1024, 1024, 0, 0, 0, 1, 4, 0, 0, 0]);
        let second = blob_icon(blob, 14, 10);
        assert_eq!(second.segs().collect::<Vec<_>>(), vec![Seg::Z]);
        assert_eq!(second.fill_mesh(), None, "an all-zero fill section is no fill");
        assert_eq!(blob_icon(blob, 0, 14).segs().collect::<Vec<_>>(), vec![Seg::M(1.0, 1.0)]);
    }

    #[test]
    fn segs_icons_iterate_their_slice_and_have_no_fill() {
        static SEGS: [Seg; 2] = [Seg::M(0.0, 0.0), Seg::L(1.0, 1.0)];
        let icon = Icon { name: "s", size: 24.0, shape: IconShape::Segs(&SEGS) };
        assert_eq!(icon.segs().collect::<Vec<_>>(), SEGS.to_vec());
        assert_eq!(icon.fill_mesh(), None);
    }

    #[test]
    fn unknown_opcode_ends_the_strokes_instead_of_panicking() {
        let blob = leak(&[3, 9, 0, 0, 0, 0, 0]);
        assert_eq!(blob_icon(blob, 0, 14).segs().count(), 0);
    }

    #[test]
    fn blob_ranges_compare_by_identity_and_print_without_bytes() {
        let blob = record();
        let a = BlobRange { blob, start: 0, len: 8 };
        assert_eq!(a, a);
        assert_ne!(a, BlobRange { start: 2, ..a });
        let copy: &'static [u8] = Box::leak(blob.to_vec().into_boxed_slice());
        assert_ne!(a, BlobRange { blob: copy, ..a }, "equal bytes in another blob are another icon");
        assert_eq!(format!("{a:?}"), "BlobRange { start: 0, len: 8 }");
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p shadcn-egui-core --jobs 4 -- --test-threads 4`
Expected: FAIL to compile (`BlobRange`, `IconShape`, `Icon::segs` not found).

- [ ] **Step 3: Implement the decoder**

`crates/shadcn-egui-core/src/blob.rs` (above the tests):

```rust
//! Reader for the icon blobs icongen writes (`tools/icongen/src/blob.rs`
//! documents the format). Icon crates hold one blob each; an `Icon` points
//! at its record with a [`BlobRange`].

use crate::icon::Seg;

const COORD_SCALE: f32 = 1024.0;
const MITER_SCALE: f32 = 8192.0;

/// One icon's record inside an icon crate's blob: `blob[start..start + len]`.
#[derive(Clone, Copy)]
pub struct BlobRange {
    pub blob: &'static [u8],
    pub start: u32,
    pub len: u32,
}

impl BlobRange {
    fn bytes(self) -> &'static [u8] {
        &self.blob[self.start as usize..(self.start + self.len) as usize]
    }
}

/// Two ranges are the same icon when they point into the same blob at the
/// same place; the bytes are never compared.
impl PartialEq for BlobRange {
    fn eq(&self, other: &Self) -> bool {
        core::ptr::eq(self.blob.as_ptr(), other.blob.as_ptr()) && self.start == other.start && self.len == other.len
    }
}

impl core::fmt::Debug for BlobRange {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlobRange").field("start", &self.start).field("len", &self.len).finish()
    }
}

/// Little-endian 16-bit reader over one record.
#[derive(Clone)]
struct Reader {
    bytes: &'static [u8],
    pos: usize,
}

impl Reader {
    fn word(&mut self) -> u16 {
        let w = u16::from_le_bytes([self.bytes[self.pos], self.bytes[self.pos + 1]]);
        self.pos += 2;
        w
    }
    fn coord(&mut self) -> f32 {
        f32::from(self.word() as i16) / COORD_SCALE
    }
    fn miter(&mut self) -> f32 {
        f32::from(self.word() as i16) / MITER_SCALE
    }
}

/// The stroke commands of one blob record, in the 24-unit box.
#[derive(Clone)]
pub struct BlobSegs {
    r: Reader,
    end: usize,
}

impl BlobSegs {
    pub(crate) fn new(range: BlobRange) -> Self {
        let mut r = Reader { bytes: range.bytes(), pos: 0 };
        let len = usize::from(r.word());
        BlobSegs { end: 2 + 2 * len, r }
    }
}

impl core::fmt::Debug for BlobSegs {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlobSegs").field("pos", &self.r.pos).field("end", &self.end).finish()
    }
}

impl Iterator for BlobSegs {
    type Item = Seg;

    fn next(&mut self) -> Option<Seg> {
        if self.r.pos >= self.end {
            return None;
        }
        let r = &mut self.r;
        Some(match r.word() {
            0 => Seg::M(r.coord(), r.coord()),
            1 => Seg::L(r.coord(), r.coord()),
            2 => Seg::Q(r.coord(), r.coord(), r.coord(), r.coord()),
            3 => Seg::C(r.coord(), r.coord(), r.coord(), r.coord(), r.coord(), r.coord()),
            4 => Seg::Z,
            _ => {
                // Not written by icongen: stop drawing rather than crash a frame.
                r.pos = self.end;
                return None;
            }
        })
    }
}

/// A pre-triangulated fill in the 24-unit box, plus per contour each
/// vertex and its outward miter (length ≤ 2; zero where no fringe belongs).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FillMesh {
    pub vertices: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
    pub contours: Vec<Vec<([f32; 2], [f32; 2])>>,
}

pub(crate) fn fill_mesh(range: BlobRange) -> Option<FillMesh> {
    let mut r = Reader { bytes: range.bytes(), pos: 0 };
    let stroke_len = usize::from(r.word());
    r.pos += 2 * stroke_len;
    let (nv, ni, nc) = (usize::from(r.word()), usize::from(r.word()), usize::from(r.word()));
    if nv == 0 && nc == 0 {
        return None;
    }
    let vertices: Vec<[f32; 2]> = (0..nv).map(|_| [r.coord(), r.coord()]).collect();
    let indices: Vec<u16> = (0..ni).map(|_| r.word()).collect();
    let contours: Vec<Vec<([f32; 2], [f32; 2])>> = (0..nc)
        .map(|_| {
            let len = usize::from(r.word());
            (0..len).map(|_| ([r.coord(), r.coord()], [r.miter(), r.miter()])).collect()
        })
        .collect();
    Some(FillMesh { vertices, indices, contours })
}
```

If `p6-icon-sizes.md` records **branch (b)**, read the index count as
`let raw = r.word(); let ni = usize::from(raw & 0x7fff);` and, when
`raw & 0x8000 != 0`, read `ni` bytes (two per word, low byte first, the last
word padded) and rebuild `idx = prev + byte - 128`; add a unit test with a
hand-made byte-delta record next to `blob_record_decodes_strokes_and_fill`.

In `crates/shadcn-egui-core/src/icon.rs`, replace the `Icon` struct (keep
`Seg` and Phase 4's `IconSlots` as they are) with:

```rust
use crate::blob::{self, BlobRange, BlobSegs, FillMesh};

/// A generated icon: its name (for screen readers), the side of its square
/// view box, and where its path data lives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Icon {
    pub name: &'static str,
    pub size: f32,
    pub shape: IconShape,
}

/// Where an icon's path data lives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IconShape {
    /// Path commands in a Rust slice (icongen `--out`); strokes only.
    Segs(&'static [Seg]),
    /// A record in an icon crate's blob (icongen `--crate-dir`): strokes and
    /// an optional fill, in the 24-unit box.
    Blob(BlobRange),
}

impl Icon {
    /// The stroke commands, in icon units. A non-empty sequence starts with `Seg::M`.
    pub fn segs(&self) -> SegIter {
        match self.shape {
            IconShape::Segs(s) => SegIter::Slice(s.iter()),
            IconShape::Blob(r) => SegIter::Words(BlobSegs::new(r)),
        }
    }

    /// The pre-triangulated fill; `None` for stroke-only icons.
    pub fn fill_mesh(&self) -> Option<FillMesh> {
        match self.shape {
            IconShape::Segs(_) => None,
            IconShape::Blob(r) => blob::fill_mesh(r),
        }
    }
}

/// The stroke commands of an [`Icon`], from either storage.
#[derive(Clone, Debug)]
pub enum SegIter {
    Slice(core::slice::Iter<'static, Seg>),
    Words(BlobSegs),
}

impl Iterator for SegIter {
    type Item = Seg;

    fn next(&mut self) -> Option<Seg> {
        match self {
            SegIter::Slice(it) => it.next().copied(),
            SegIter::Words(w) => w.next(),
        }
    }
}
```

`crates/shadcn-egui-core/src/lib.rs`: keep `mod blob;` and extend the
re-exports (keep whatever Phase 2/4 added, e.g. the font types and `IconSlots`):

```rust
pub use blob::{BlobRange, BlobSegs, FillMesh};
pub use icon::{Icon, IconShape, IconSlots, Seg, SegIter};
```

Run: `cargo test -p shadcn-egui-core --jobs 4 -- --test-threads 4`
Expected: PASS.

- [ ] **Step 4: Move every `Icon` literal to `shape`**

```bash
grep -rn "segs:" crates/ --include=*.rs | grep -v "tests/icons/"
```

Expected: the three consts in `crates/shadcn-egui/src/components/icon.rs`
(and any other hand-written `Icon { .. segs: .. }` Phase 4 added). Change
each `segs: &[…],` to `shape: IconShape::Segs(&[…]),`, e.g.:

```rust
pub const CHECK: Icon = Icon {
    name: "check",
    size: 24.0,
    shape: IconShape::Segs(&[Seg::M(20.0, 6.0), Seg::L(9.0, 17.0), Seg::L(4.0, 12.0)]),
};
```

In `crates/shadcn-egui/src/components/icon.rs` the re-export becomes
`pub use shadcn_egui_core::{Icon, IconShape, IconSlots, Seg};` (Phase 4 added `IconSlots`), `paint_icon`'s loop
becomes `for seg in icon.segs() {` with `match seg {` (was `match *seg`), and
its precondition comment reads `/// Precondition: `icon.segs()` starts with `Seg::M` (icongen guarantees it).`

- [ ] **Step 5: icongen `--out` emits `IconShape::Segs`**

`tools/icongen/tests/convert.rs`, in `names_and_module_render`, the two
assertions become:

```rust
    assert!(src.contains("use crate::shadcn::components::icon::{Icon, IconShape, Seg::*};"), "{src}");
    assert!(src.contains("pub const X: Icon = Icon {\n    name: \"x\",\n    size: 24.0,\n    shape: IconShape::Segs(&[\n"), "{src}");
```

Run: `cargo test --manifest-path tools/icongen/Cargo.toml --jobs 4 --test convert -- --test-threads 4`
Expected: FAIL (`names_and_module_render`).

In `render_module` (`tools/icongen/src/lib.rs`): the `use` line becomes
`"\nuse {module_path}::{{Icon, IconShape, Seg::*}};\n"`, the per-icon header
`"\npub const {}: Icon = Icon {{\n    name: {:?},\n    size: {},\n    shape: IconShape::Segs(&[\n"`,
and the closing `"    ],\n};\n"` becomes `"    ]),\n};\n"`.

Run the same command. Expected: PASS. Then regenerate the `--out` fixture:

```bash
cargo run --manifest-path tools/icongen/Cargo.toml --jobs 4 -- \
  --out crates/shadcn-egui/tests/icons/lucide.rs \
  --module-path shadcn_egui::components::icon \
  --provenance "Lucide v1.54.0 (ISC), test fixtures from tools/icongen/tests/fixtures" \
  tools/icongen/tests/fixtures/{mic,circle-alert,settings,x,power,monitor}.svg
```

- [ ] **Step 6: Generate the blob fixture crate and pin it with a test**

`tools/icongen/tests/fixture_crate.rs`:

```rust
//! The blob fixture under crates/shadcn-egui/tests/icons/fixture is generated
//! by the command in skills/egui-shadcn/references/icons.md; this test fails
//! when icongen's output for it changes and the fixture was not regenerated.

use icongen::{convert, parse_slots, render_crate};

const NAMES: [&str; 6] = ["big-box", "mixed", "ring", "square-hole", "star-evenodd", "x"];
const SLOTS: &str = "check=mixed,chevron_down=ring,close=x";
const PROVENANCE: &str = "icongen test fixtures (tools/icongen/tests/fixtures); x.svg is Lucide (ISC)";

#[test]
fn committed_fixture_crate_is_current() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let icons: Vec<_> = NAMES
        .iter()
        .map(|n| {
            let svg = std::fs::read_to_string(format!("{manifest}/tests/fixtures/{n}.svg")).expect("fixture");
            convert(n, &svg).expect("converts")
        })
        .collect();
    let (src, blob) = render_crate(&icons, &parse_slots(SLOTS).expect("slots"), PROVENANCE).expect("renders");
    let dir = format!("{manifest}/../../crates/shadcn-egui/tests/icons/fixture/src");
    let committed_src = std::fs::read_to_string(format!("{dir}/lib.rs")).expect("committed lib.rs");
    let committed_blob = std::fs::read(format!("{dir}/icons.bin")).expect("committed icons.bin");
    assert!(committed_src == src, "fixture lib.rs is stale: regenerate it (references/icons.md)");
    assert!(committed_blob == blob, "fixture icons.bin is stale: regenerate it (references/icons.md)");
}
```

Run: `cargo test --manifest-path tools/icongen/Cargo.toml --jobs 4 --test fixture_crate -- --test-threads 4`
Expected: FAIL (`committed lib.rs` not found). Generate:

```bash
cargo run --manifest-path tools/icongen/Cargo.toml --jobs 4 -- \
  --crate-dir crates/shadcn-egui/tests/icons/fixture \
  --slots check=mixed,chevron_down=ring,close=x \
  --provenance "icongen test fixtures (tools/icongen/tests/fixtures); x.svg is Lucide (ISC)" \
  tools/icongen/tests/fixtures/{big-box,mixed,ring,square-hole,star-evenodd,x}.svg
```

Re-run the test. Expected: PASS. (Cargo only auto-discovers `tests/*.rs` and
`tests/*/main.rs`, so `tests/icons/fixture/src/lib.rs` is not a test target.)

- [ ] **Step 7: Integration tests that decode the generated blob**

`crates/shadcn-egui/tests/icon_fill.rs`:

```rust
#[allow(dead_code)]
#[path = "icons/fixture/src/lib.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "icons/lucide.rs"]
mod lucide;

use shadcn_egui::{Icon, Seg};

fn coords(s: Seg) -> Vec<f32> {
    match s {
        Seg::M(x, y) | Seg::L(x, y) => vec![x, y],
        Seg::Q(a, b, x, y) => vec![a, b, x, y],
        Seg::C(a, b, c, d, x, y) => vec![a, b, c, d, x, y],
        Seg::Z => vec![],
    }
}

fn area(f: &shadcn_egui::FillMesh) -> f32 {
    f.indices
        .chunks(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| f.vertices[i as usize]);
            ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0
        })
        .sum()
}

#[test]
fn blob_strokes_match_the_slice_form() {
    let blob: Vec<Seg> = fixture::X.segs().collect();
    let slice: Vec<Seg> = lucide::X.segs().collect();
    assert_eq!(blob.len(), slice.len());
    for (b, s) in blob.iter().zip(&slice) {
        for (x, y) in coords(*b).iter().zip(coords(*s)) {
            assert!((x - y).abs() <= 0.5 / 1024.0, "{b:?} vs {s:?}");
        }
    }
    assert_eq!(fixture::X.fill_mesh(), None, "a stroke icon has no fill");
}

#[test]
fn blob_fills_decode_with_their_area_and_contours() {
    let f = fixture::SQUARE_HOLE.fill_mesh().expect("filled");
    assert!((area(&f) - 220.0).abs() < 0.01, "area {}", area(&f));
    assert_eq!(f.contours.len(), 2);
    assert!(f.indices.iter().all(|&i| usize::from(i) < f.vertices.len()));
    let mixed = fixture::MIXED;
    assert!(mixed.segs().count() > 0 && mixed.fill_mesh().is_some(), "strokes and fill in one record");
    let xs: Vec<f32> = fixture::BIG_BOX.fill_mesh().expect("filled").vertices.iter().flat_map(|v| *v).collect();
    assert!(xs.iter().all(|v| (3.0..=21.0).contains(v)), "256 view box normalised to 24 units");
}

#[test]
fn generated_consts_point_into_one_blob() {
    assert_eq!(fixture::ALL.len(), 6);
    assert_eq!(fixture::SLOTS.check, fixture::MIXED);
    assert_eq!(fixture::SLOTS.close.name, "x");
    let icon: &Icon = fixture::ALL[0];
    assert_eq!(icon.size, 24.0);
}
```

Run: `cargo test -p shadcn-egui --jobs 4 --test icon_fill --test icon -- --test-threads 4`
Expected: PASS.

- [ ] **Step 8: Gates (no snapshot may change) and commit**

Run every gate of the index (both feature sets, clippy, tools, drift).
Expected: all green and `git status --short crates/shadcn-egui/tests/snapshots`
prints nothing.

```bash
git add crates/shadcn-egui-core crates/shadcn-egui/src/components/icon.rs \
  crates/shadcn-egui/tests/icon_fill.rs crates/shadcn-egui/tests/icons \
  tools/icongen
git add -u crates/shadcn-egui
git commit -m "feat(core): Icon holds an IconShape; blob records decode without dependencies

Icon.segs becomes Icon.shape: either a Seg slice (icongen --out) or a
range into an icon crate's blob. Icon::segs() iterates both forms and
Icon::fill_mesh() decodes the pre-triangulated fill. A generated fixture
crate pins the format between icongen and the decoder.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6.4: Painter: fill mesh with an outward-only fringe

**Files:**
- Modify: `crates/shadcn-egui/src/components/icon.rs` (`paint_icon`, new private `fill_shape`, module doc)
- Modify: `crates/shadcn-egui/tests/icon_fill.rs`
- Create: `crates/shadcn-egui/tests/snapshots/icon_fill_edge_a100.png`, `icon_fill_edge_a50.png`

**Interfaces:**
- Consumes: `Icon::fill_mesh()`, `FillMesh` (Task 6.3); `Painter::pixels_per_point()`, `egui::Mesh::{colored_vertex, add_triangle}`, `Shape::mesh`.
- Produces: `paint_icon(painter, rect, icon, color, stroke_width)` unchanged in signature; it now paints the fill mesh first, then the strokes. Review Focus 4 test `filled_icon_has_no_seam_at_fractional_scale`.

- [ ] **Step 1: Write the failing tests**

Append to `crates/shadcn-egui/tests/icon_fill.rs`:

```rust
use egui::{pos2, vec2, Color32, Id, LayerId, Order, Rect, Shape};
use egui_kittest::Harness;
use shadcn_egui::components::icon::paint_icon;

/// Paint one icon at pixels_per_point 1 and return the flattened shapes.
fn painted(icon: &Icon, rect: Rect) -> Vec<Shape> {
    let ctx = egui::Context::default();
    let out = ctx.run_ui(egui::RawInput::default(), |ui| {
        paint_icon(&ui.painter_at(Rect::EVERYTHING), rect, icon, Color32::RED, 2.0);
    });
    fn walk(s: &Shape, out: &mut Vec<Shape>) {
        match s {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            s => out.push(s.clone()),
        }
    }
    let mut shapes = Vec::new();
    out.shapes.iter().for_each(|c| walk(&c.shape, &mut shapes));
    shapes
}

#[test]
fn fill_is_one_mesh_with_an_outward_fringe() {
    // Scale 2: the outer corner (4, 4) of square-hole lands at (8, 8).
    let shapes = painted(&fixture::SQUARE_HOLE, Rect::from_min_size(pos2(0.0, 0.0), vec2(48.0, 48.0)));
    let meshes: Vec<_> = shapes.iter().filter_map(|s| if let Shape::Mesh(m) = s { Some(m.clone()) } else { None }).collect();
    assert_eq!(meshes.len(), 1, "one mesh per icon");
    let mesh = &meshes[0];
    let fill = fixture::SQUARE_HOLE.fill_mesh().expect("filled");
    let nv = fill.vertices.len();
    let ring: usize = fill.contours.iter().map(Vec::len).sum();
    assert_eq!(mesh.vertices.len(), nv + 2 * ring);
    assert_eq!(mesh.indices.len(), fill.indices.len() + 6 * ring, "two fringe triangles per edge");
    assert!(mesh.vertices[..nv].iter().all(|v| v.color == Color32::RED));
    let k = mesh.vertices[nv..]
        .chunks(2)
        .position(|p| p[0].pos == pos2(8.0, 8.0))
        .expect("the corner is a contour vertex");
    let (inner, outer) = (mesh.vertices[nv + 2 * k], mesh.vertices[nv + 2 * k + 1]);
    assert_eq!(inner.color, Color32::RED);
    assert_eq!(outer.color, Color32::TRANSPARENT);
    assert!((outer.pos - pos2(7.0, 7.0)).length() < 1e-3, "one pixel out along the miter: {:?}", outer.pos);
}

#[test]
fn fill_is_painted_before_strokes() {
    let shapes = painted(&fixture::MIXED, Rect::from_min_size(pos2(0.0, 0.0), vec2(24.0, 24.0)));
    let mesh = shapes.iter().position(|s| matches!(s, Shape::Mesh(_))).expect("the fill");
    let path = shapes.iter().position(|s| matches!(s, Shape::Path(_))).expect("the stroke");
    assert!(mesh < path, "fill first, then strokes: {shapes:?}");
}

/// Review Focus 4: a translucent filled icon shows no darker seam where the
/// fill meets the fringe and no lighter gap inside, also at a fractional scale.
#[test]
fn filled_icon_has_no_seam_at_fractional_scale() {
    for ppp in [1.0_f32, 1.25] {
        let mut h = Harness::builder().with_size(vec2(40.0, 40.0)).with_pixels_per_point(ppp).build_ui(|ui| {
            let page = ui.ctx().viewport_rect();
            ui.ctx().layer_painter(LayerId::background()).rect_filled(page, 0.0, Color32::WHITE);
            let p = ui.ctx().layer_painter(LayerId::new(Order::Foreground, Id::new("icon")));
            let rect = Rect::from_min_size(pos2(8.0, 8.0), vec2(24.0, 24.0));
            paint_icon(&p, rect, &fixture::RING, Color32::RED.gamma_multiply(0.5), 2.0);
        });
        h.run();
        let img = h.render().expect("render");
        let g = |x: u32, y: u32| i32::from(img.get_pixel(x, y).0[1]);
        let px = |v: f32| ((8.0 + v) * ppp) as u32;
        // Icon (12, 4) lies between the ring's edge (radius 10) and its hole (8..16).
        let interior = g(px(12.0), px(4.0));
        assert!((120..=136).contains(&interior), "ppp {ppp}: interior green {interior}");
        for y in 0..img.height() {
            for x in 0..img.width() {
                let v = g(x, y);
                assert!(v >= interior - 3, "ppp {ppp}: ({x}, {y}) green {v} < interior {interior} - 3: seam");
                let (ix, iy) = ((x as f32 + 0.5) / ppp - 8.0, (y as f32 + 0.5) / ppp - 8.0);
                let r = ((ix - 12.0).powi(2) + (iy - 12.0).powi(2)).sqrt();
                let near_hole = (7.0..=17.0).contains(&ix) && (7.0..=17.0).contains(&iy);
                if r < 9.0 && !near_hole {
                    assert!((v - interior).abs() <= 3, "ppp {ppp}: ({x}, {y}) green {v} vs interior {interior}: gap");
                }
            }
        }
    }
}

fn zoomed(name: &str, alpha: f32) {
    let mut h = Harness::builder().with_size(vec2(96.0, 32.0)).with_pixels_per_point(8.0).build_ui(|ui| {
        let page = ui.ctx().viewport_rect();
        ui.ctx().layer_painter(LayerId::background()).rect_filled(page, 0.0, Color32::WHITE);
        let p = ui.ctx().layer_painter(LayerId::new(Order::Foreground, Id::new("icons")));
        let color = Color32::from_rgb(9, 9, 11).gamma_multiply(alpha);
        for (i, icon) in [&fixture::RING, &fixture::STAR_EVENODD, &fixture::MIXED].into_iter().enumerate() {
            let rect = Rect::from_min_size(pos2(4.0 + 32.0 * i as f32, 4.0), vec2(24.0, 24.0));
            paint_icon(&p, rect, icon, color, 2.0);
        }
    });
    h.run();
    h.snapshot(name);
}

#[test]
fn filled_icon_edges_zoomed_opaque() {
    zoomed("icon_fill_edge_a100", 1.0);
}

#[test]
fn filled_icon_edges_zoomed_half() {
    zoomed("icon_fill_edge_a50", 0.5);
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p shadcn-egui --jobs 4 --test icon_fill -- --test-threads 4`
Expected: FAIL — `fill_is_one_mesh_with_an_outward_fringe` ("one mesh per
icon", found 0), `fill_is_painted_before_strokes`, the seam test (the fill
is not painted, interior green ≈ 255), and both snapshots (missing baseline).

- [ ] **Step 3: Implement**

`crates/shadcn-egui/src/components/icon.rs` — module doc, replace the first
paragraph with:

```rust
//! Vector icons painted with egui shapes, no image loader. Icon data is
//! generated ahead of time by `tools/icongen` (strokes as path commands,
//! fills as pre-triangulated meshes); this module only paints it.
```

Imports become:

```rust
use crate::{FillMesh, Theme};
use egui::epaint::{CubicBezierShape, PathShape, QuadraticBezierShape};
use egui::{Color32, Mesh, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2, Widget};
```

`paint_icon` doc and start (the rest of the body stays):

```rust
/// Paint `icon` scaled into the square `rect`: its fill first (if any), then
/// its strokes. `stroke_width` is in icon units (Lucide's own is 2.0) and
/// scales with the icon.
///
/// Precondition: `icon.segs()` starts with `Seg::M` (icongen guarantees it).
pub fn paint_icon(painter: &Painter, rect: Rect, icon: &Icon, color: Color32, stroke_width: f32) {
    let scale = rect.width() / icon.size;
    let map = |x: f32, y: f32| rect.min + Vec2::new(x, y) * scale;
    let stroke = Stroke::new(stroke_width * scale, color);
    let cap_r = stroke.width / 2.0;

    let mut shapes = Vec::new();
    if let Some(fill) = icon.fill_mesh() {
        shapes.push(fill_shape(&fill, map, color, 1.0 / painter.pixels_per_point()));
    }
    let mut points: Vec<Pos2> = Vec::new();
```

(delete the old `let mut shapes = Vec::new();` line that followed `cap_r`).

Below `paint_icon` add:

```rust
/// The fill as one mesh: its triangles in `color`, plus per contour a strip
/// from the edge (in `color`) to `fringe` points outward along each vertex's
/// miter (transparent). Like epaint's own feathering but outward only, so the
/// strip never overlaps the fill and a translucent colour shows no darker
/// seam; `fringe` is one physical pixel.
fn fill_shape(fill: &FillMesh, map: impl Fn(f32, f32) -> Pos2, color: Color32, fringe: f32) -> Shape {
    let mut mesh = Mesh::default();
    for &[x, y] in &fill.vertices {
        mesh.colored_vertex(map(x, y), color);
    }
    mesh.indices.extend(fill.indices.iter().map(|&i| u32::from(i)));
    for contour in &fill.contours {
        if contour.is_empty() {
            continue;
        }
        let base = mesh.vertices.len() as u32;
        let n = contour.len() as u32;
        for &([x, y], [mx, my]) in contour {
            let p = map(x, y);
            mesh.colored_vertex(p, color);
            mesh.colored_vertex(p + Vec2::new(mx, my) * fringe, Color32::TRANSPARENT);
        }
        let mut i0 = n - 1;
        for i1 in 0..n {
            let (in0, out0, in1, out1) = (base + 2 * i0, base + 2 * i0 + 1, base + 2 * i1, base + 2 * i1 + 1);
            mesh.add_triangle(in0, in1, out1);
            mesh.add_triangle(in0, out1, out0);
            i0 = i1;
        }
    }
    Shape::mesh(mesh)
}
```

- [ ] **Step 4: Run the tests and write the new baselines**

```bash
cargo test -p shadcn-egui --jobs 4 --test icon_fill -- --test-threads 4 --skip zoomed
UPDATE_SNAPSHOTS=1 cargo test -p shadcn-egui --jobs 4 --test icon_fill zoomed -- --test-threads 4
cargo test --workspace --jobs 4 -- --test-threads 4
git status --short crates/shadcn-egui/tests/snapshots
```

Expected: the first run passes (structure and seam tests); the second
writes two PNGs; the full run passes; `git status` lists only the two new
`icon_fill_edge_*.png` (no existing snapshot changed: all existing icons are
stroke-only and take the unchanged path).

- [ ] **Step 5: Look at the PNGs yourself**

Open `icon_fill_edge_a100.png` and `icon_fill_edge_a50.png` (768×256 px,
each physical pixel at 8× zoom). Check: every fill edge, curved and
straight, has a one-pixel soft border on the outside and none on the inside;
the ring's hole edge is soft towards the hole; at 50 % no line along the
edge is darker than the interior, and no triangle seams inside the fill;
the star's centre is empty; the `mixed` dot sits above an even stroke.

- [ ] **Step 6: STOP — user reviews the PNGs**

Show the user the two PNGs and ask them to review and approve them. Do not
continue until they approve; apply their requested changes first.

- [ ] **Step 7: Commit**

```bash
git add crates/shadcn-egui/src/components/icon.rs crates/shadcn-egui/tests/icon_fill.rs \
  crates/shadcn-egui/tests/snapshots/icon_fill_edge_a100.png crates/shadcn-egui/tests/snapshots/icon_fill_edge_a50.png
git commit -m "feat(icon): paint filled icons as a mesh with an outward-only fringe

egui anti-aliases only shapes it tessellates; a raw mesh has hard edges,
and a stroke over the edge would double-cover the fill under alpha. The
painter adds a one-physical-pixel strip outward along icongen's miters,
fading to transparent. Zoomed snapshots at full and half opacity and a
pixel test at pixels_per_point 1.0 and 1.25 pin it. Reviewed by the user.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6.5: `shadcn-egui-icons-lucide` (full set) replaces the built-in consts

**Files:**
- Create: `tools/icongen/scripts/generate-crates.sh`
- Create: `crates/shadcn-egui-icons-lucide/Cargo.toml`, `LICENSE-ICONS.txt` (copied by the script)
- Generate: `crates/shadcn-egui-icons-lucide/src/{lib.rs,icons.bin}`
- Modify: root `Cargo.toml` (`[workspace.dependencies]`)
- Modify: `crates/shadcn-egui/Cargo.toml` (`[dependencies]`)
- Modify: `crates/shadcn-egui/src/icons.rs`, `crates/shadcn-egui/src/theme.rs` (`Theme::new`), `crates/shadcn-egui/src/preset/mod.rs` (icon fallback), `crates/shadcn-egui/src/components/icon.rs` (delete `CHECK`, `CHEVRON_DOWN`, `CLOSE`, `LUCIDE_SLOTS`)
- Modify: every user of those consts (`crates/shadcn-egui/tests/{components.rs,theme.rs,preset_fallbacks.rs}`, and whatever the grep in Step 5 finds)
- Create: `crates/shadcn-egui/tests/icon_sets.rs`

**Interfaces:**
- Consumes: icongen `--crate-dir` (Task 6.1), `IconShape::Blob` (Task 6.3), Phase 4's `icons::slots(IconLibrary) -> Option<IconSlots>`, `Theme.icons`, `IconLibrary`.
- Produces:
  - crate `shadcn_egui_icons_lucide`: `pub const <NAME>: Icon` per Lucide icon (e.g. `CHECK`, `CHEVRON_DOWN`, `X`, `MIC`), `pub const ALL: &[&Icon]`, `pub const SLOTS: IconSlots` (check / chevron-down / x).
  - **NEW INTERFACE:** `shadcn_egui::icons::lucide` = `pub use shadcn_egui_icons_lucide as lucide;` (and in Task 6.6 `icons::{tabler, phosphor, remixicon, hugeicons}` behind their features), so users reach every icon without naming the icon crate.
  - **NEW INTERFACE:** `tools/icongen/scripts/generate-crates.sh <scratch dir> [lib…]` — reads `SETS.tsv`, downloads each pinned package into `<scratch>/npm/<pkg>-<v>/`, writes `crates/shadcn-egui-icons-<lib>/src/{lib.rs,icons.bin}` and `LICENSE-ICONS.txt` for each `ship=yes` lib asked for (all when none given).
  - `Theme::new(..).icons == shadcn_egui::icons::lucide::SLOTS`.

- [ ] **Step 1: Write the failing tests**

`crates/shadcn-egui/tests/icon_sets.rs`:

```rust
mod common;

use shadcn_egui::style::Style;
use shadcn_egui::{icons, Icon, IconLibrary, Palette, Seg, Theme};

/// Every icon of a set decodes to something drawable with in-range indices.
fn every_icon_decodes(all: &[&Icon]) {
    assert!(!all.is_empty());
    for icon in all {
        assert_eq!(icon.size, 24.0, "{}", icon.name);
        let fill = icon.fill_mesh();
        assert!(icon.segs().count() > 0 || fill.is_some(), "{} draws nothing", icon.name);
        if let Some(f) = fill {
            assert_eq!(f.indices.len() % 3, 0, "{}", icon.name);
            assert!(f.indices.iter().all(|&i| usize::from(i) < f.vertices.len()), "{}: index out of range", icon.name);
        }
    }
}

#[test]
fn lucide_is_the_default_icon_set() {
    assert_eq!(Theme::new(Style::vega(), Palette::light(), 10.0).icons, icons::lucide::SLOTS);
    assert_eq!(icons::slots(IconLibrary::Lucide), Some(icons::lucide::SLOTS));
    let s = icons::lucide::SLOTS;
    assert_eq!((s.check.name, s.chevron_down.name, s.close.name), ("check", "chevron-down", "x"));
}

#[test]
fn lucide_slots_keep_the_built_in_paths() {
    let segs = |i: &Icon| i.segs().collect::<Vec<_>>();
    assert_eq!(segs(&icons::lucide::CHECK), vec![Seg::M(20.0, 6.0), Seg::L(9.0, 17.0), Seg::L(4.0, 12.0)]);
    assert_eq!(segs(&icons::lucide::CHEVRON_DOWN), vec![Seg::M(6.0, 9.0), Seg::L(12.0, 15.0), Seg::L(18.0, 9.0)]);
    assert_eq!(
        segs(&icons::lucide::X),
        vec![Seg::M(18.0, 6.0), Seg::L(6.0, 18.0), Seg::M(6.0, 6.0), Seg::L(18.0, 18.0)]
    );
}

#[test]
fn every_lucide_icon_decodes() {
    every_icon_decodes(icons::lucide::ALL);
    assert!(icons::lucide::ALL.len() > 1000, "the full set, not a sample");
}
```

Run: `cargo test -p shadcn-egui --jobs 4 --test icon_sets -- --test-threads 4`
Expected: FAIL to compile (`icons::lucide` not found).

- [ ] **Step 2: Write the generation script**

`tools/icongen/scripts/generate-crates.sh` (mode 755):

```bash
#!/usr/bin/env bash
# Regenerate the icon crates from the pinned npm packages in tools/icongen/SETS.tsv.
#
#   tools/icongen/scripts/generate-crates.sh <scratch dir> [lib...]
#
# Without libs, every row with ship=yes is generated. Downloads land in
# <scratch dir>/npm/<package>-<version>/ and are read as data; only the
# Hugeicons package is executed (imported by hugeicons-svg.mjs), because its
# icons exist only as JavaScript data.
set -euo pipefail
repo=$(cd "$(dirname "$0")/../../.." && pwd)
scratch=${1:?usage: generate-crates.sh <scratch dir> [lib...]}
shift
want=" ${*:-lucide tabler phosphor remixicon hugeicons} "

slots() {
  case $1 in
    lucide) echo check=check,chevron_down=chevron-down,close=x ;;
    tabler) echo check=check,chevron_down=selector,close=x ;;
    phosphor) echo check=check,chevron_down=caret-down,close=x ;;
    remixicon) echo check=check-line,chevron_down=arrow-down-s-line,close=close-line ;;
    hugeicons) echo check=tick-02,chevron_down=unfold-more,close=cancel-01 ;;
    *) echo "unknown lib $1" >&2; exit 1 ;;
  esac
}

title() {
  case $1 in
    lucide) echo "Lucide" ;;
    tabler) echo "Tabler Icons, outline set" ;;
    phosphor) echo "Phosphor Icons, regular weight" ;;
    remixicon) echo "Remix Icon, pinned to its last Apache-2.0 release and never updated" ;;
    hugeicons) echo "Hugeicons, free stroke-rounded set" ;;
  esac
}

fetch() { # fetch <npm name> <version> -> prints the extraction dir
  local dir="$scratch/npm/$(echo "$1" | tr / _)-$2"
  if [ ! -d "$dir/package" ]; then
    mkdir -p "$dir"
    local tgz
    tgz=$(curl -sf "https://registry.npmjs.org/$1/$2" | jq -r .dist.tarball)
    curl -sfL "$tgz" | tar -xz -C "$dir"
  fi
  echo "$dir"
}

cargo build --release --manifest-path "$repo/tools/icongen/Cargo.toml" --jobs 4
icongen=$(cargo metadata --manifest-path "$repo/tools/icongen/Cargo.toml" --format-version 1 --no-deps | jq -r .target_directory)/release/icongen

tail -n +2 "$repo/tools/icongen/SETS.tsv" | while IFS=$'\t' read -r lib package version glob license ship; do
  case $want in *" $lib "*) ;; *) continue ;; esac
  if [ "$ship" != yes ]; then
    echo "skip $lib (ship=$ship)"
    continue
  fi
  dir=$(fetch "$package" "$version")
  if [ "$lib" = hugeicons ]; then
    svgdir="$scratch/hugeicons-svg-$version"
    node "$repo/tools/icongen/scripts/hugeicons-svg.mjs" "$dir" "$svgdir"
    files=("$svgdir"/*.svg)
  else
    # shellcheck disable=SC2206  # the glob from SETS.tsv is meant to expand
    files=("$dir"/$glob)
  fi
  crate="$repo/crates/shadcn-egui-icons-$lib"
  "$icongen" --crate-dir "$crate" --slots "$(slots "$lib")" \
    --provenance "$(title "$lib") $version ($license), from the npm package $package." "${files[@]}"
  lic=$(find "$dir/package" -maxdepth 1 -type f -iname 'licen[cs]e*' | head -1)
  [ -n "$lic" ] || { echo "$lib: no licence file in $dir/package" >&2; exit 1; }
  cp "$lic" "$crate/LICENSE-ICONS.txt"
done
```

Run: `bash -n tools/icongen/scripts/generate-crates.sh && chmod 755 tools/icongen/scripts/generate-crates.sh`
Expected: no output.

- [ ] **Step 3: The crate manifest and the workspace entries**

`crates/shadcn-egui-icons-lucide/Cargo.toml`:

```toml
[package]
name = "shadcn-egui-icons-lucide"
description = "Lucide icons for shadcn-egui"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license = "MIT AND ISC"
repository.workspace = true

[dependencies]
shadcn-egui-core.workspace = true
```

Root `Cargo.toml`, `[workspace.dependencies]`:

```toml
shadcn-egui-icons-lucide = { path = "crates/shadcn-egui-icons-lucide", version = "0.4.0" }
```

`crates/shadcn-egui/Cargo.toml`, `[dependencies]` (next to the Inter font
crate, outside any `# BEGIN fontgen` block):

```toml
shadcn-egui-icons-lucide.workspace = true
```

- [ ] **Step 4: Generate**

```bash
tools/icongen/scripts/generate-crates.sh "$SCRATCH" lucide
head -5 crates/shadcn-egui-icons-lucide/src/lib.rs
grep -c '^pub const .*: Icon = icon(' crates/shadcn-egui-icons-lucide/src/lib.rs
ls -l crates/shadcn-egui-icons-lucide/src/icons.bin crates/shadcn-egui-icons-lucide/LICENSE-ICONS.txt
```

Expected: the header names `Lucide <version> (ISC), from the npm package lucide-static.`;
the const count equals the number of SVGs in the package; both files exist.

- [ ] **Step 5: Switch the crate to the generated set**

`crates/shadcn-egui/src/icons.rs` — at the top, below the module doc:

```rust
/// Lucide, the default icon set (always available).
pub use shadcn_egui_icons_lucide as lucide;
```

and the Lucide arm of `slots` returns `Some(lucide::SLOTS)`.

Then find every user of the built-in consts:

```bash
grep -rnE "LUCIDE_SLOTS|icon::(\{[^}]*)?\b(CHECK|CLOSE|CHEVRON_DOWN)\b" crates/shadcn-egui/src crates/shadcn-egui/tests crates/shadcn-egui/examples --include=*.rs
```

and change each:
- `LUCIDE_SLOTS` (in `Theme::new` in `theme.rs`, in the icon fallback of `Preset::theme` in `preset/mod.rs`, in tests) → `crate::icons::lucide::SLOTS` inside the crate, `shadcn_egui::icons::lucide::SLOTS` in tests and examples (`icons::lucide` is the re-export of `shadcn_egui_icons_lucide`, so this is the crate's `SLOTS`).
- `components::icon::{CHECK, CHEVRON_DOWN, CLOSE}` → `icons::lucide::{CHECK, CHEVRON_DOWN, X}`; in `tests/components.rs` the import becomes `use shadcn_egui::icons::lucide::X as CLOSE;`.

The known users from Phase 4, changed in this step (the test bodies stay
as they are):

- `crates/shadcn-egui/src/theme.rs`, in `Theme::new`:
  `icons: crate::components::icon::LUCIDE_SLOTS,` → `icons: crate::icons::lucide::SLOTS,`
- `crates/shadcn-egui/src/icons.rs`, in `slots`:
  `IconLibrary::Lucide => Some(crate::components::icon::LUCIDE_SLOTS),` → `IconLibrary::Lucide => Some(lucide::SLOTS),`
- `crates/shadcn-egui/src/preset/mod.rs`: delete `use crate::components::icon::LUCIDE_SLOTS;`; in `Preset::theme` the icon step becomes

  ```rust
          let slots = crate::icons::slots(self.icons).unwrap_or_else(|| {
              log::warn!("{}", crate::icons::fallback_warning(self.icons));
              crate::icons::lucide::SLOTS
          });
  ```

- `crates/shadcn-egui/tests/theme.rs` (Task 4.4): the import
  `use shadcn_egui::components::icon::{CHECK, CHEVRON_DOWN, CLOSE, LUCIDE_SLOTS};` becomes

  ```rust
  use shadcn_egui::icons::lucide::{CHECK, CHEVRON_DOWN, SLOTS as LUCIDE_SLOTS, X as CLOSE};
  ```

- `crates/shadcn-egui/tests/preset_fallbacks.rs` (Task 4.6): the import
  `use shadcn_egui::components::icon::LUCIDE_SLOTS;` becomes

  ```rust
  use shadcn_egui::icons::lucide::SLOTS as LUCIDE_SLOTS;
  ```

Delete from `crates/shadcn-egui/src/components/icon.rs` the consts `CLOSE`,
`CHECK`, `CHEVRON_DOWN` and `LUCIDE_SLOTS` with their doc comments, and the
now unused imports. Re-run the grep. Expected: no hits; `cargo build --workspace --all-targets --jobs 4`
reports any use the grep missed (e.g. `use super::*` inside `components/`).

- [ ] **Step 6: Run the tests, existing snapshots must not change**

```bash
cargo test -p shadcn-egui --jobs 4 --test icon_sets -- --test-threads 4
cargo test -p shadcn-egui --lib --jobs 4 small_stack -- --test-threads 4
cargo test --workspace --jobs 4 -- --test-threads 4
git status --short crates/shadcn-egui/tests/snapshots
```

Expected: all pass, including the 256 KB stack tests (`IconSlots` grows by a
few dozen bytes per icon, nothing else in `Theme` changes); `git status`
prints nothing. The three slot icons have integer coordinates, so the
quantised blob gives the same points and the checkbox, select and dialog
snapshots stay identical. If `lucide_slots_keep_the_built_in_paths` fails,
print the pinned `check.svg`/`chevron-down.svg`/`x.svg` and compare: a
changed upstream path is a snapshot change, and the snapshot rule applies
(STOP for the user's review).

- [ ] **Step 7: Gates and commit**

Run every gate of the index (both feature sets, clippy, tools, drift).

```bash
git add Cargo.toml Cargo.lock crates/shadcn-egui-icons-lucide tools/icongen/scripts/generate-crates.sh \
  crates/shadcn-egui/tests/icon_sets.rs
git add -u crates/shadcn-egui
git commit -m "feat: shadcn-egui-icons-lucide ships the full Lucide set; built-in icon consts removed

Every Lucide icon is a generated const in one blob; SLOTS feeds
Theme::new and the Lucide fallback. components::icon::{CHECK,
CHEVRON_DOWN, CLOSE, LUCIDE_SLOTS} are gone; use icons::lucide::{CHECK,
CHEVRON_DOWN, X, SLOTS}. generate-crates.sh rebuilds icon crates from the
pinned npm packages in SETS.tsv.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6.6: Tabler, Phosphor, Remixicon (pinned) and Hugeicons crates, features, slot snapshots

**Files:**
- Create: `crates/shadcn-egui-icons-{tabler,phosphor,remixicon}/Cargo.toml`; generated `src/{lib.rs,icons.bin}` and `LICENSE-ICONS.txt`
- Create (only if `SETS.tsv` has `hugeicons … ship=yes`): `tools/icongen/scripts/hugeicons-svg.mjs`, `crates/shadcn-egui-icons-hugeicons/…` as above
- Modify: root `Cargo.toml`, `crates/shadcn-egui/Cargo.toml` (optional deps, features)
- Modify: `crates/shadcn-egui/src/icons.rs` (re-exports, `slots` arms, Hugeicons arm of `fallback_warning`)
- Modify: `crates/shadcn-egui/src/preset/mod.rs` (only if Task 6.5 left it differing from the code in Step 5)
- Modify: `crates/shadcn-egui/tests/icon_sets.rs`
- Create: `crates/shadcn-egui/tests/snapshots/icon_slots_{lucide,tabler,phosphor,remixicon[,hugeicons]}_vega_light.png`

**Interfaces:**
- Consumes: `generate-crates.sh` (Task 6.5); `Preset` with the `.icons(IconLibrary)` setter, `Preset::theme(Mode)`; `common::snapshot_with_themes(name, size, themes, ui_fn, prepare)`; `common::logcap::warnings`.
- Produces:
  - crates `shadcn_egui_icons_{tabler,phosphor,remixicon[,hugeicons]}`, each with every icon, `ALL`, `SLOTS` (slot icons per index D table).
  - features `icons-tabler`, `icons-phosphor`, `icons-remixicon`, (`icons-hugeicons`), `icons-all`.
  - `shadcn_egui::icons::{tabler, phosphor, remixicon, hugeicons}` re-exports behind the features.
  - Extends Phase 4's `pub(crate) fn icons::fallback_warning(lib: IconLibrary) -> String` (Task 4.5) with the Hugeicons "not available (licence)" arm when `ship=no`.
  - `tools/icongen/scripts/hugeicons-svg.mjs <pkg dir> <out dir>` (index D).

- [ ] **Step 1: Write the failing tests**

Append to `crates/shadcn-egui/tests/icon_sets.rs`:

```rust
use egui::vec2;
use shadcn_egui::components::button::{Button, ButtonSize, ButtonVariant};
use shadcn_egui::components::checkbox::Checkbox;
use shadcn_egui::components::icon::IconView;
use shadcn_egui::components::select::Select;
use shadcn_egui::{layout, Mode, Preset};

/// The slot icons of `lib` as the components draw them, vega light.
fn slot_snapshot(lib: IconLibrary, name: &str) {
    let theme = Preset::default().style(shadcn_egui::StyleName::Vega).icons(lib).theme(Mode::Light);
    let slots = theme.icons;
    common::snapshot_with_themes(
        name,
        vec2(360.0, 96.0),
        vec![("vega_light", theme)],
        move |ui| {
            layout::vstack(ui, 12.0, |ui| {
                layout::row(ui, 16.0, |ui| {
                    for icon in [&slots.check, &slots.chevron_down, &slots.close] {
                        ui.add(IconView::new(icon));
                        ui.add(IconView::new(icon).size(24.0));
                    }
                });
                layout::row(ui, 12.0, |ui| {
                    ui.add(Checkbox::new(&mut true));
                    ui.add(Select::new("slots", &mut 0, &["Option"]));
                    ui.add(Button::new("Close").icon(&slots.close).size(ButtonSize::Icon).variant(ButtonVariant::Ghost));
                });
            });
        },
        |_| {},
    );
}

#[test]
fn lucide_slot_snapshot() {
    slot_snapshot(IconLibrary::Lucide, "icon_slots_lucide");
}

#[cfg(feature = "icons-tabler")]
#[test]
fn tabler_set() {
    every_icon_decodes(icons::tabler::ALL);
    assert_eq!(icons::slots(IconLibrary::Tabler), Some(icons::tabler::SLOTS));
    let s = icons::tabler::SLOTS;
    assert_eq!((s.check.name, s.chevron_down.name, s.close.name), ("check", "selector", "x"));
    slot_snapshot(IconLibrary::Tabler, "icon_slots_tabler");
}

#[cfg(feature = "icons-phosphor")]
#[test]
fn phosphor_set() {
    every_icon_decodes(icons::phosphor::ALL);
    assert_eq!(icons::slots(IconLibrary::Phosphor), Some(icons::phosphor::SLOTS));
    let s = icons::phosphor::SLOTS;
    assert_eq!((s.check.name, s.chevron_down.name, s.close.name), ("check", "caret-down", "x"));
    assert!(s.check.fill_mesh().is_some(), "Phosphor regular is a filled outline");
    slot_snapshot(IconLibrary::Phosphor, "icon_slots_phosphor");
}

#[cfg(feature = "icons-remixicon")]
#[test]
fn remixicon_set() {
    every_icon_decodes(icons::remixicon::ALL);
    assert_eq!(icons::slots(IconLibrary::Remixicon), Some(icons::remixicon::SLOTS));
    let s = icons::remixicon::SLOTS;
    assert_eq!((s.check.name, s.chevron_down.name, s.close.name), ("check-line", "arrow-down-s-line", "close-line"));
    assert!(s.close.fill_mesh().is_some(), "Remix Icon is a filled outline");
    slot_snapshot(IconLibrary::Remixicon, "icon_slots_remixicon");
}
```

Hugeicons, **if `SETS.tsv` says `ship=yes`**, append:

```rust
#[cfg(feature = "icons-hugeicons")]
#[test]
fn hugeicons_set() {
    every_icon_decodes(icons::hugeicons::ALL);
    assert_eq!(icons::slots(IconLibrary::Hugeicons), Some(icons::hugeicons::SLOTS));
    let s = icons::hugeicons::SLOTS;
    assert_eq!((s.check.name, s.chevron_down.name, s.close.name), ("tick-02", "unfold-more", "cancel-01"));
    slot_snapshot(IconLibrary::Hugeicons, "icon_slots_hugeicons");
}
```

**if `ship=no`**, append instead:

```rust
#[test]
fn hugeicons_is_not_available_for_licence_reasons() {
    let (theme, lines) = common::logcap::warnings(|| Preset::default().icons(IconLibrary::Hugeicons).theme(Mode::Light));
    assert_eq!(theme.icons, icons::lucide::SLOTS);
    assert_eq!(lines, vec![r#"shadcn-egui: icon library "hugeicons" is not available (licence); using "lucide""#.to_owned()]);
    assert_eq!(icons::slots(IconLibrary::Hugeicons), None);
}
```

Run: `cargo test -p shadcn-egui --all-features --jobs 4 --test icon_sets -- --test-threads 4`
Expected: FAIL to compile (`icons::tabler` etc. not found; with `ship=no`,
the Hugeicons test fails on the warning text if Phase 4 printed the
"needs the feature" line).

- [ ] **Step 2: Hugeicons SVG export (only if `ship=yes`)**

`tools/icongen/scripts/hugeicons-svg.mjs`:

```js
// Write one SVG per Hugeicons icon export, for icongen.
//   node hugeicons-svg.mjs <package dir> <out dir>
// <package dir> holds the extracted npm tarball (its `package/` folder).
// Each icon export is an array of [tag, attrs] pairs (P0c notes).
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const [pkgDir, outDir] = process.argv.slice(2);
if (!pkgDir || !outDir) {
  console.error("usage: hugeicons-svg.mjs <package dir> <out dir>");
  process.exit(2);
}
const root = join(pkgDir, "package");
const meta = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const dot = meta.exports?.["."];
const fromExports = typeof dot === "string" ? dot : (dot?.import?.default ?? dot?.import ?? dot?.default);
const entry = meta.module ?? fromExports ?? meta.main;
const mod = await import(pathToFileURL(join(root, entry)).href);

// Tick02Icon -> tick-02, UnfoldMoreIcon -> unfold-more
const kebab = (s) => s.replace(/([a-z])([A-Z0-9])/g, "$1-$2").replace(/([0-9])([A-Z])/g, "$1-$2").toLowerCase();
const attr = (k) => k.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase());
const esc = (v) => String(v).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");

mkdirSync(outDir, { recursive: true });
const seen = new Set();
for (const [exportName, data] of Object.entries(mod)) {
  if (!exportName.endsWith("Icon") || !Array.isArray(data)) continue;
  const name = kebab(exportName.slice(0, -"Icon".length));
  if (seen.has(name)) continue; // aliases of the same icon
  seen.add(name);
  const body = data
    .map(([tag, attrs]) => {
      const a = Object.entries(attrs)
        .filter(([k]) => k !== "key")
        .map(([k, v]) => `${attr(k)}="${esc(v)}"`)
        .join(" ");
      return `<${tag} ${a}/>`;
    })
    .join("");
  writeFileSync(
    join(outDir, `${name}.svg`),
    `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor">${body}</svg>\n`,
  );
}
console.error(`hugeicons-svg: wrote ${seen.size} icons to ${outDir}`);
```

Run: `node tools/icongen/scripts/hugeicons-svg.mjs "$SCRATCH/npm/@hugeicons_core-free-icons-<v>" "$SCRATCH/hugeicons-check" && ls "$SCRATCH/hugeicons-check" | grep -E '^(tick-02|unfold-more|cancel-01)\.svg$'`
(`<v>` from `SETS.tsv`.) Expected: three lines. The stroke width in the
data (1.5) is ignored: the painter strokes with `Metrics::icon_stroke`
(2.0), as upstream renders Hugeicons with `strokeWidth={2}`.

- [ ] **Step 3: Crate manifests and workspace entries**

One manifest per shipped set, in the form of Task 6.5 Step 3, with:

| Crate | `description` | `license` |
|---|---|---|
| `shadcn-egui-icons-tabler` | `Tabler Icons (outline) for shadcn-egui` | `MIT` |
| `shadcn-egui-icons-phosphor` | `Phosphor Icons (regular) for shadcn-egui` | `MIT` |
| `shadcn-egui-icons-remixicon` | `Remix Icon (last Apache-2.0 release) for shadcn-egui` | `MIT AND Apache-2.0` |
| `shadcn-egui-icons-hugeicons` (ship=yes only) | `Hugeicons (free) for shadcn-egui` | `MIT` if `SETS.tsv` says MIT, else `MIT AND <SPDX from SETS.tsv>` |

For example `crates/shadcn-egui-icons-tabler/Cargo.toml`:

```toml
[package]
name = "shadcn-egui-icons-tabler"
description = "Tabler Icons (outline) for shadcn-egui"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license = "MIT"
repository.workspace = true

[dependencies]
shadcn-egui-core.workspace = true
```

Root `Cargo.toml`, `[workspace.dependencies]`:

```toml
shadcn-egui-icons-tabler = { path = "crates/shadcn-egui-icons-tabler", version = "0.4.0" }
shadcn-egui-icons-phosphor = { path = "crates/shadcn-egui-icons-phosphor", version = "0.4.0" }
shadcn-egui-icons-remixicon = { path = "crates/shadcn-egui-icons-remixicon", version = "0.4.0" }
shadcn-egui-icons-hugeicons = { path = "crates/shadcn-egui-icons-hugeicons", version = "0.4.0" }  # ship=yes only
```

`crates/shadcn-egui/Cargo.toml` — `[dependencies]` (outside the fontgen markers):

```toml
shadcn-egui-icons-tabler = { workspace = true, optional = true }
shadcn-egui-icons-phosphor = { workspace = true, optional = true }
shadcn-egui-icons-remixicon = { workspace = true, optional = true }
shadcn-egui-icons-hugeicons = { workspace = true, optional = true }  # ship=yes only
```

`[features]` (outside the fontgen markers):

```toml
icons-tabler = ["dep:shadcn-egui-icons-tabler"]
icons-phosphor = ["dep:shadcn-egui-icons-phosphor"]
icons-remixicon = ["dep:shadcn-egui-icons-remixicon"]
icons-hugeicons = ["dep:shadcn-egui-icons-hugeicons"]  # ship=yes only
icons-all = ["icons-tabler", "icons-phosphor", "icons-remixicon", "icons-hugeicons"]  # drop icons-hugeicons when ship=no
```

- [ ] **Step 4: Generate the crates**

```bash
tools/icongen/scripts/generate-crates.sh "$SCRATCH" tabler phosphor remixicon hugeicons
for c in crates/shadcn-egui-icons-*/; do
  printf '%s %s icons, %s bytes\n' "$c" "$(grep -c '^pub const .*: Icon = icon(' "$c/src/lib.rs")" "$(stat -c %s "$c/src/icons.bin")"
done
head -3 crates/shadcn-egui-icons-remixicon/src/lib.rs
```

Expected: one line per crate (Hugeicons skipped with `skip hugeicons
(ship=no)` when not shipped); the Remixicon header names the pinned version
and "(Apache-2.0)". Check `crates/shadcn-egui-icons-remixicon/LICENSE-ICONS.txt`
starts with the Apache License 2.0, not the Remix Icon License.

- [ ] **Step 5: `icons.rs` re-exports, slots and the fallback line**

`crates/shadcn-egui/src/icons.rs` after this step (merge with Phase 4's doc
comments; the function names and Lucide behaviour are unchanged):

```rust
//! Icon sets: Lucide always, the others behind their `icons-<lib>` feature.

use crate::{IconLibrary, IconSlots};

/// Lucide, the default icon set (always available).
pub use shadcn_egui_icons_lucide as lucide;
/// Tabler Icons, outline set.
#[cfg(feature = "icons-tabler")]
pub use shadcn_egui_icons_tabler as tabler;
/// Phosphor Icons, regular weight.
#[cfg(feature = "icons-phosphor")]
pub use shadcn_egui_icons_phosphor as phosphor;
/// Remix Icon, pinned to its last Apache-2.0 release.
#[cfg(feature = "icons-remixicon")]
pub use shadcn_egui_icons_remixicon as remixicon;
/// Hugeicons, free set. (ship=yes only)
#[cfg(feature = "icons-hugeicons")]
pub use shadcn_egui_icons_hugeicons as hugeicons;

/// The slot icons of `lib`, or `None` when its feature is off (or the
/// library is not shipped).
pub fn slots(lib: IconLibrary) -> Option<IconSlots> {
    match lib {
        IconLibrary::Lucide => Some(lucide::SLOTS),
        #[cfg(feature = "icons-tabler")]
        IconLibrary::Tabler => Some(tabler::SLOTS),
        #[cfg(feature = "icons-phosphor")]
        IconLibrary::Phosphor => Some(phosphor::SLOTS),
        #[cfg(feature = "icons-remixicon")]
        IconLibrary::Remixicon => Some(remixicon::SLOTS),
        #[cfg(feature = "icons-hugeicons")]
        IconLibrary::Hugeicons => Some(hugeicons::SLOTS),
        #[allow(unreachable_patterns)]
        _ => None,
    }
}

/// The warning `Preset::theme` logs when `slots(lib)` is `None`.
pub(crate) fn fallback_warning(lib: IconLibrary) -> String {
    match lib {
        // ship=no only: Hugeicons' licence forbids republishing its icons.
        IconLibrary::Hugeicons => {
            r#"shadcn-egui: icon library "hugeicons" is not available (licence); using "lucide""#.to_owned()
        }
        _ => format!(r#"shadcn-egui: icon library "{0}" needs the "icons-{0}" feature; using "lucide""#, lib.id()),
    }
}
```

With `ship=yes`: drop the `IconLibrary::Hugeicons` arm of `fallback_warning`
(then a missing `icons-hugeicons` feature gets the "needs the feature" line).
With `ship=no`: drop the `hugeicons` re-export and its `slots` arm.

In `Preset::theme` (`crates/shadcn-egui/src/preset/mod.rs`), the icon step
reads as follows (Phase 4 Task 4.5 with Task 6.5's `LUCIDE_SLOTS` switch;
verify, change nothing else):

```rust
        let slots = crate::icons::slots(self.icons).unwrap_or_else(|| {
            log::warn!("{}", crate::icons::fallback_warning(self.icons));
            crate::icons::lucide::SLOTS
        });
```

- [ ] **Step 6: Run the tests and write the new baselines**

```bash
cargo test --workspace --jobs 4 -- --test-threads 4 --skip slot_snapshot --skip _set
UPDATE_SNAPSHOTS=1 cargo test -p shadcn-egui --all-features --jobs 4 --test icon_sets -- --test-threads 4
cargo test --workspace --all-features --jobs 4 -- --test-threads 4
cargo test --workspace --jobs 4 -- --test-threads 4
git status --short crates/shadcn-egui/tests/snapshots
```

Expected: every run passes (the default-features run runs Phase 4's
`an_icon_library_without_its_feature_falls_back_to_lucide` (the
`--all-features` run prints `skipped: icons-tabler is enabled`) and, for `ship=no`,
`hugeicons_is_not_available_for_licence_reasons`); `git status` lists only
the new `icon_slots_*_vega_light.png` files, one per shipped set plus Lucide.

- [ ] **Step 7: Look at the PNGs yourself**

Open each `icon_slots_*.png`. Check against the upstream look of each set:
Tabler's trigger is the up/down `selector`, Phosphor's a filled `caret-down`
outline, Remix's `arrow-down-s-line`, Hugeicons' `unfold-more`; the filled
sets (Phosphor, Remix) have smooth edges and visible holes (Phosphor `x`
and `check` are outlines, not solid blobs); the checkbox tick sits centred
in the box; the icons are the same visual size as the Lucide row.

- [ ] **Step 8: STOP — user reviews the PNGs**

Show the user the new PNGs (`git status --short crates/shadcn-egui/tests/snapshots`)
and ask them to review and approve them. Do not continue until they approve;
apply their requested changes first.

- [ ] **Step 9: Gates and commit**

Run every gate of the index (both feature sets, clippy, tools, drift).

```bash
git add Cargo.toml Cargo.lock crates/shadcn-egui-icons-* tools/icongen/scripts \
  crates/shadcn-egui/tests/icon_sets.rs crates/shadcn-egui/tests/snapshots/icon_slots_*.png
git add -u crates/shadcn-egui
git commit -m "feat: Tabler, Phosphor and Remix Icon crates behind icons-<lib> features

Each set ships in full as one blob crate with SLOTS mapped to the icon
upstream uses in Checkbox, Select and Dialog. Remix Icon is built from its
last Apache-2.0 release and never updated (later releases forbid icon-pack
redistribution). <Hugeicons: shipped as icons-hugeicons | not shipped: its
licence forbids republishing; the preset falls back to Lucide with a
licence warning.> Slot snapshots reviewed by the user.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

(Keep exactly one of the two Hugeicons sentences in the message.)

---

### Task 6.7: Package size gate, docs, changelog

**Files:**
- Create: `scripts/check-package-size.sh`
- Modify: `skills/egui-shadcn/references/icons.md`
- Modify: `CHANGES.md`

**Interfaces:**
- Consumes: every workspace crate; `cargo metadata`, `jq`.
- Produces: `scripts/check-package-size.sh` — runs `cargo package --workspace --no-verify --allow-dirty --jobs 4` once, prints each crate's `.crate` size, exits 1 if any is above the limit. **NEW INTERFACE:** env `PACKAGE_SIZE_LIMIT` (bytes, default `8388608` = 8 MB) overrides the limit, so the failure path can be tested.

- [ ] **Step 1: See the gate missing**

Run: `scripts/check-package-size.sh`
Expected: `No such file or directory`.

- [ ] **Step 2: Write the script**

`scripts/check-package-size.sh` (mode 755):

```bash
#!/usr/bin/env bash
# Fail when a workspace crate packages to more than 8 MB. crates.io refuses
# a .crate above 10 MB; the margin leaves room for growth between releases.
#
#   scripts/check-package-size.sh            # limit 8 MB
#   PACKAGE_SIZE_LIMIT=1000 scripts/check-package-size.sh
set -euo pipefail
cd "$(dirname "$0")/.."
limit=${PACKAGE_SIZE_LIMIT:-$((8 * 1024 * 1024))}
meta=$(cargo metadata --format-version 1 --no-deps)
target=$(jq -r .target_directory <<<"$meta")

cargo package --workspace --no-verify --allow-dirty --jobs 4

fail=0
while read -r name version; do
  crate="$target/package/$name-$version.crate"
  size=$(stat -c %s "$crate")
  printf '%10d  %s\n' "$size" "$name"
  if [ "$size" -gt "$limit" ]; then
    echo "too large: $name is $size bytes, the limit is $limit" >&2
    fail=1
  fi
done < <(jq -r '.packages[] | "\(.name) \(.version)"' <<<"$meta")
exit "$fail"
```

- [ ] **Step 3: Run it both ways**

```bash
chmod 755 scripts/check-package-size.sh
PACKAGE_SIZE_LIMIT=1000 scripts/check-package-size.sh; echo "exit $?"
scripts/check-package-size.sh; echo "exit $?"
```

Expected: the first run lists every crate and prints `too large: …` lines,
then `exit 1`; the second lists every crate under 8388608 bytes and prints
`exit 0`. Copy the second listing into the commit message body. If a crate
is above 8 MB here although Task 6.2 measured it below, **STOP** and ask the
user (index D, branch c).

- [ ] **Step 4: Update the icon reference**

`skills/egui-shadcn/references/icons.md`:

1. Replace the bullet `- Filled shapes, embedded images, text and non-square view boxes are errors, …`
   with:

   ```markdown
   - Filled shapes are supported in icon crates (`--crate-dir`): icongen
     flattens them at 0.02 units, triangulates them under their fill rule and
     stores outward edge miters; the painter draws the mesh with a
     one-pixel outward fringe, so edges are smooth and a faded icon shows no
     darker seam. `--out` modules hold strokes only and reject fills.
   - Embedded images, text and non-square view boxes are errors, not silent
     omissions: the generator names the file and stops.
   ```

2. Append a section:

   ````markdown
   ## Icon crates (maintainers)

   The crates `shadcn-egui-icons-<lib>` are generated from the pinned npm
   packages in `tools/icongen/SETS.tsv`; never edit their `src/`:

   ```bash
   tools/icongen/scripts/generate-crates.sh <scratch dir> [lib...]
   ```

   Each crate holds every icon as a `pub const` (a range into one
   `icons.bin` blob), `ALL`, and `SLOTS` for the checkbox tick, select
   trigger and close button. Remix Icon stays at its last Apache-2.0
   release. After changing icongen, regenerate the test fixtures:

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
   ````

3. In the "Use" section, the `ui.add(Button::new("Close").icon(&icons::X)…` line stays; add below the code block:

   ```markdown
   With the crate, every icon of the theme's sets is a const:
   `shadcn_egui::icons::lucide::MIC`, `shadcn_egui::icons::tabler::HOME`
   (feature `icons-tabler`). The components draw `Theme.icons` (the slots).
   ```

Run: `grep -n "CHECK\b\|CHEVRON_DOWN\|icon::CLOSE" -r skills/ README.md`
Expected: no hit that names `components::icon::{CHECK,CHEVRON_DOWN,CLOSE}`;
change any hit to `icons::lucide::{CHECK,CHEVRON_DOWN,X}`.

- [ ] **Step 5: Changelog**

`CHANGES.md`, under `## [Unreleased]` (create the `### Added` /
`### Changed` subsection if missing; no release header):

`### Added`:

```markdown
- The checkbox tick, select arrow and dialog close button follow the
  preset's icon library: Lucide by default, Tabler, Phosphor and Remix Icon
  with the features `icons-tabler`, `icons-phosphor` and `icons-remixicon`
  (`icons-all` for all). Every icon of each set is available as a constant,
  for example `shadcn_egui::icons::tabler::HOME`.
- Filled icons (Phosphor, Remix Icon) are drawn with smooth edges, also
  when faded for a disabled widget or a translucent menu.
```

Add, depending on `SETS.tsv`, one of:

```markdown
- Hugeicons is available with the feature `icons-hugeicons`.
```

```markdown
- A preset that asks for Hugeicons shows Lucide icons and logs
  `icon library "hugeicons" is not available (licence)`: its licence does
  not allow republishing the icons.
```

`### Changed`:

```markdown
- Remix Icon stays at version <pinned version from SETS.tsv>, its last
  release under the Apache-2.0 licence; later releases forbid
  redistribution as an icon pack.
- `components::icon::CHECK`, `CHEVRON_DOWN` and `CLOSE` are gone: use
  `shadcn_egui::icons::lucide::CHECK`, `CHEVRON_DOWN` and `X`. Icon modules
  generated with an older icongen must be regenerated, because an icon now
  holds a `shape` instead of `segs`.
```

(Write the actual pinned version into the Remix entry.)

- [ ] **Step 6: Phase gates and commit**

Run every gate of the index, now including `scripts/check-package-size.sh`:

```bash
cargo test --workspace --jobs 4 -- --test-threads 4
cargo test --workspace --all-features --jobs 4 -- --test-threads 4
cargo clippy --workspace --all-targets --jobs 4 -- -D warnings
cargo clippy --workspace --all-targets --all-features --jobs 4 -- -D warnings
cargo test --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- --test-threads 4
cargo clippy --manifest-path tools/stylegen/Cargo.toml --all-targets --jobs 4 -- -D warnings
cargo test --manifest-path tools/icongen/Cargo.toml --jobs 4 -- --test-threads 4
cargo clippy --manifest-path tools/icongen/Cargo.toml --all-targets --jobs 4 -- -D warnings
cargo run --manifest-path tools/stylegen/Cargo.toml --jobs 4 -- drift "$UP" "$UP_REACT" .
scripts/check-package-size.sh
```

Expected: all pass; drift prints `drift: clean`; the size gate exits 0.

```bash
git add scripts/check-package-size.sh skills/egui-shadcn/references/icons.md CHANGES.md
git commit -m "build: package size gate; docs and changelog for icon crates

scripts/check-package-size.sh packages the workspace once and fails above
8 MB per crate (crates.io refuses 10 MB). Sizes at this commit:
<paste the listing from Step 3>

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
