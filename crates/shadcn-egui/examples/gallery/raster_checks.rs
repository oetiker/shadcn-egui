//! Property checks of the rasterizer: damage rendering equals a full
//! redraw, on random scenes and on the real gallery, and the fill rule
//! blends each pixel of a mesh once.
use super::*;
use egui::epaint::Mesh;
use egui::{pos2, Pos2, Rect};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    /// Uniform in `lo..hi`.
    fn f(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next() % 1_000_000) as f32 / 1_000_000.0 * (hi - lo)
    }
    fn u(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[derive(Clone)]
struct Tri {
    p: [Pos2; 3],
    c: [Color32; 3],
    clip: Rect,
}

fn frame_prims(tris: &[Tri]) -> Vec<ClippedPrimitive> {
    // Group consecutive triangles with the same clip into one mesh, as egui does.
    let mut out: Vec<ClippedPrimitive> = Vec::new();
    for t in tris {
        let need_new = match out.last() {
            Some(p) => p.clip_rect != t.clip,
            None => true,
        };
        if need_new {
            out.push(ClippedPrimitive {
                clip_rect: t.clip,
                primitive: Primitive::Mesh(Mesh::default()),
            });
        }
        let Primitive::Mesh(m) = &mut out.last_mut().unwrap().primitive else {
            unreachable!()
        };
        let base = m.vertices.len() as u32;
        for i in 0..3 {
            m.vertices.push(Vertex {
                pos: t.p[i],
                uv: egui::epaint::WHITE_UV,
                color: t.c[i],
            });
        }
        m.indices.extend([base, base + 1, base + 2]);
    }
    out
}

fn rand_tri(r: &mut Rng, w: f32, h: f32) -> Tri {
    let cx = r.f(0.0, w);
    let cy = r.f(0.0, h);
    let s = r.f(1.0, 20.0);
    let p = [0, 1, 2].map(|_| pos2(cx + r.f(-s, s), cy + r.f(-s, s)));
    let a = [255u8, 128, 40][r.u(3)];
    let c0 = Color32::from_rgba_unmultiplied(r.u(256) as u8, r.u(256) as u8, r.u(256) as u8, a);
    let c = if r.u(2) == 0 {
        [c0; 3]
    } else {
        [0, 1, 2].map(|_| {
            Color32::from_rgba_unmultiplied(r.u(256) as u8, r.u(256) as u8, r.u(256) as u8, a)
        })
    };
    let clip = if r.u(4) == 0 {
        Rect::from_min_max(
            pos2(r.f(0.0, w / 2.0), r.f(0.0, h / 2.0)),
            pos2(r.f(w / 2.0, w), r.f(h / 2.0, h)),
        )
    } else {
        Rect::EVERYTHING
    };
    Tri { p, c, clip }
}

/// Damage rendering equals a full redraw over random edits of a scene.
#[test]
fn damage_equals_full_redraw_on_random_scenes() {
    let (w, h) = (97usize, 61usize);
    for seed in 1..400u64 {
        let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let mut tris: Vec<Tri> = (0..20)
            .map(|_| rand_tri(&mut r, w as f32, h as f32))
            .collect();
        let mut canvas = Canvas::new(w, h);
        let mut inc = Rasterizer::with_white_atlas();
        for step in 0..24 {
            let ppp = [1.0f32, 1.0, 1.25, 1.5, 2.0][(seed as usize) % 5];
            let prims = frame_prims(&tris);
            inc.render(&mut canvas, &prims, ppp, 0x00ff_ffff);
            let mut full = Canvas::new(w, h);
            Rasterizer::with_white_atlas().render(&mut full, &prims, ppp, 0x00ff_ffff);
            if canvas.pixels != full.pixels {
                let bad = canvas
                    .pixels
                    .iter()
                    .zip(&full.pixels)
                    .filter(|(a, b)| a != b)
                    .count();
                panic!("seed {seed} step {step}: {bad} px differ");
            }
            match r.u(6) {
                0 if !tris.is_empty() => {
                    let i = r.u(tris.len());
                    tris.remove(i);
                }
                1 => {
                    let i = r.u(tris.len() + 1);
                    tris.insert(i, rand_tri(&mut r, w as f32, h as f32));
                }
                2 if tris.len() > 1 => {
                    let i = r.u(tris.len());
                    let j = r.u(tris.len());
                    tris.swap(i, j);
                }
                3 if !tris.is_empty() => {
                    let i = r.u(tris.len());
                    let d = r.f(-3.0, 3.0);
                    for p in &mut tris[i].p {
                        p.x += d;
                    }
                }
                4 if !tris.is_empty() => {
                    // duplicate a triangle elsewhere in the list
                    let i = r.u(tris.len());
                    let t = tris[i].clone();
                    let j = r.u(tris.len() + 1);
                    tris.insert(j, t);
                }
                _ if tris.len() > 2 => {
                    // move a triangle in the order
                    let i = r.u(tris.len());
                    let t = tris.remove(i);
                    let j = r.u(tris.len() + 1);
                    tris.insert(j, t);
                }
                _ => {}
            }
        }
    }
}

/// A jittered grid split in triangles: each pixel is blended exactly once.
#[test]
fn the_fill_rule_blends_a_jittered_mesh_once() {
    let (mut skipped, mut tested) = (0, 0);
    for seed in 1..2000u64 {
        let mut r = Rng(seed.wrapping_mul(0x2545_F491_4F6C_DD1D) | 1);
        let (w, h) = (64usize, 64usize);
        let n = 8;
        let ppp = [1.0f32, 1.5, 2.0, 1.333][seed as usize % 4];
        let mut pts = vec![vec![pos2(0.0, 0.0); n + 1]; n + 1];
        for (j, row) in pts.iter_mut().enumerate() {
            for (i, p) in row.iter_mut().enumerate() {
                let jx = if i == 0 || i == n {
                    0.0
                } else {
                    r.f(-2.0, 2.0)
                };
                let jy = if j == 0 || j == n {
                    0.0
                } else {
                    r.f(-2.0, 2.0)
                };
                // Some vertices on exact half-pixel and integer coordinates.
                let snap = |v: f32| {
                    if on_grid(seed, i, j) {
                        (v * 2.0).round() / 2.0
                    } else {
                        v
                    }
                };
                *p = pos2(
                    snap(4.0 + i as f32 * 6.0 + jx),
                    snap(4.0 + j as f32 * 6.0 + jy),
                );
            }
        }
        let half = Color32::from_black_alpha(128);
        let mut mesh = Mesh::default();
        for j in 0..n {
            for i in 0..n {
                let a = pts[j][i];
                let b = pts[j][i + 1];
                let c = pts[j + 1][i + 1];
                let d = pts[j + 1][i];
                let base = mesh.vertices.len() as u32;
                for p in [a, b, c, d] {
                    mesh.vertices.push(Vertex {
                        pos: p,
                        uv: egui::epaint::WHITE_UV,
                        color: half,
                    });
                }
                if (i + j) % 2 == 0 {
                    mesh.indices
                        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                } else {
                    mesh.indices
                        .extend([base, base + 1, base + 3, base + 1, base + 2, base + 3]);
                }
            }
        }
        let mut signs = std::collections::HashSet::new();
        for t in mesh.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|k| mesh.vertices[t[k] as usize].pos);
            let o = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            signs.insert(o > 0.0);
        }
        // A jitter that folds a cell is no triangulation.
        if signs.len() > 1 {
            skipped += 1;
            continue;
        }
        tested += 1;
        let mut c = Canvas::new((w as f32 * ppp) as usize, (h as f32 * ppp) as usize);
        Rasterizer::with_white_atlas().render(
            &mut c,
            &[ClippedPrimitive {
                clip_rect: Rect::EVERYTHING,
                primitive: Primitive::Mesh(mesh),
            }],
            ppp,
            0x00ff_ffff,
        );
        let mut vals: Vec<u32> = c.pixels.clone();
        vals.sort_unstable();
        vals.dedup();
        // The background and one blend: no pixel blended twice.
        assert!(vals.len() <= 2, "seed {seed} ppp {ppp}: values {vals:x?}");
        // The mesh's outline is the square 4..52 (in points): every pixel
        // whose center lies in it is blended, none is skipped.
        let centers =
            |lo: f32, hi: f32| ((hi * ppp - 0.5).ceil() - (lo * ppp - 0.5).ceil()) as usize;
        let side = centers(4.0, 4.0 + n as f32 * 6.0);
        let blended = c.pixels.iter().filter(|&&p| p != 0x00ff_ffff).count();
        assert_eq!(blended, side * side, "seed {seed} ppp {ppp}");
    }
    assert!(tested > skipped * 10, "tested {tested}, folded {skipped}");
}

/// Whether vertex (i, j) of mesh `seed` snaps to the half-pixel grid.
fn on_grid(seed: u64, i: usize, j: usize) -> bool {
    (seed as usize + i * 7 + j * 13).is_multiple_of(3)
}

/// The real gallery: damage rendering over a stream of input equals a
/// full redraw. Trimmed for the gates; the long run is
/// `damage_equals_full_redraw_on_the_gallery_long`.
#[test]
fn damage_equals_full_redraw_on_the_gallery() {
    gallery_damage_check(640.0, 480.0, 24);
}

/// The same check at 900x700 points and 160 frames per set-up, about 15 s
/// in release: `cargo test --release -p shadcn-egui --example
/// gallery --features fonts-all,icons-all -- --ignored gallery_long`.
#[test]
#[ignore = "long; run by hand after a change to the rasterizer"]
fn damage_equals_full_redraw_on_the_gallery_long() {
    gallery_damage_check(900.0, 700.0, 160);
}

fn gallery_damage_check(wp: f32, hp: f32, frames: usize) {
    for (ppp, dark) in [(1.0f32, false), (1.5, true), (2.0, false)] {
        let ctx = egui::Context::default();
        let (mut state, _) = crate::initial_state(None, Some(if dark { "dark" } else { "light" }));
        state.preset.apply(&ctx);
        ctx.set_theme(state.mode);
        let (w, h) = ((wp * ppp) as usize, (hp * ppp) as usize);
        let mut canvas = Canvas::new(w, h);
        let mut inc = Rasterizer::default();
        let mut full_r = Rasterizer::default();
        let mut r = Rng(12345);
        let mut t = 0.0f64;
        for step in 0..frames {
            let mut events = Vec::new();
            let p = pos2(r.f(0.0, wp), r.f(0.0, hp));
            events.push(egui::Event::PointerMoved(p));
            if step % 5 == 0 {
                for pressed in [true, false] {
                    events.push(egui::Event::PointerButton {
                        pos: p,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    });
                }
            }
            if step % 9 == 0 {
                events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -r.f(50.0, 400.0)),
                    phase: egui::TouchPhase::Move,
                    modifiers: Default::default(),
                });
            }
            if step % 13 == 0 {
                events.push(egui::Event::Key {
                    key: egui::Key::Tab,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                });
            }
            t += 1.0 / 60.0;
            let raw = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(wp, hp))),
                time: Some(t),
                events,
                ..Default::default()
            };
            ctx.set_pixels_per_point(ppp);
            let out = ctx.run_ui(raw, |ui| shadcn_egui::reference::gallery_ui(ui, &mut state));
            let ppp_out = out.pixels_per_point;
            let prims = ctx.tessellate(out.shapes, ppp_out);
            inc.set_textures(&out.textures_delta);
            full_r.set_textures(&out.textures_delta);
            inc.render(&mut canvas, &prims, ppp_out, 0x0011_2233);
            full_r.full_damage = true;
            let mut full = Canvas::new(w, h);
            full_r.render(&mut full, &prims, ppp_out, 0x0011_2233);
            inc.free_textures(&out.textures_delta);
            full_r.free_textures(&out.textures_delta);
            let bad = canvas
                .pixels
                .iter()
                .zip(&full.pixels)
                .filter(|(a, b)| a != b)
                .count();
            assert_eq!(bad, 0, "ppp {ppp} step {step}: {bad} px differ");
        }
    }
}
