//! Frame cost of the gallery screen in each exported style: the median time
//! and the heap allocations per frame, in a headless `egui::Context` on one
//! thread. A manual tool, not a test:
//! `cargo run --release --example bench_frame --jobs 4 [-- <frames>]`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use shadcn_egui::reference::{gallery_ui, GalleryState};
use shadcn_egui::{Preset, StyleName};

/// Counts every allocation and reallocation; frees are not counted.
struct Counting;

static ALLOCS: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

const STYLES: [(&str, StyleName); 7] = [
    ("vega", StyleName::Vega),
    ("nova", StyleName::Nova),
    ("maia", StyleName::Maia),
    ("lyra", StyleName::Lyra),
    ("mira", StyleName::Mira),
    ("luma", StyleName::Luma),
    ("rhea", StyleName::Rhea),
];

/// Frames run before measuring: fonts reach the atlas, layouts settle.
const WARMUP: usize = 20;

// No time and no events: every frame measured is the same static frame.
fn input() -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1280.0, 1420.0),
        )),
        ..Default::default()
    }
}

fn main() {
    let frames: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(3000);
    println!("| style | median µs | allocs/frame |  ({frames} frames)");
    for (name, style) in STYLES {
        let ctx = egui::Context::default();
        let mut state = GalleryState::new(Preset::default().style(style), egui::Theme::Light);
        for _ in 0..WARMUP {
            let _ = ctx.run_ui(input(), |ui| gallery_ui(ui, &mut state));
        }
        let mut times: Vec<Duration> = Vec::with_capacity(frames);
        let before = ALLOCS.load(Ordering::Relaxed);
        for _ in 0..frames {
            let raw = input();
            let start = Instant::now();
            let _ = ctx.run_ui(raw, |ui| gallery_ui(ui, &mut state));
            times.push(start.elapsed());
        }
        let allocs = ALLOCS.load(Ordering::Relaxed) - before;
        times.sort();
        let median = times[times.len() / 2];
        println!(
            "| {name} | {:.1} | {:.0} |",
            median.as_secs_f64() * 1e6,
            allocs as f64 / frames as f64
        );
    }
}
