//! Every component under a live preset picker:
//! `cargo run --example gallery --features fonts-all,icons-all [-- <preset code> [dark|light]]`.

use shadcn_egui::reference::{gallery_ui, GalleryState};
use shadcn_egui::{Mode, Preset};

struct GalleryApp {
    state: GalleryState,
}

impl eframe::App for GalleryApp {
    // gallery_ui applies the preset itself, only when a picker changed it.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        gallery_ui(ui, &mut self.state);
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let preset = match args.get(1) {
        Some(code) => Preset::from_code(code).unwrap_or_else(|e| {
            eprintln!("gallery: {code}: {e}; using the default preset");
            Preset::default()
        }),
        None => Preset::default(),
    };
    let mode = if args.get(2).map(String::as_str) == Some("dark") {
        Mode::Dark
    } else {
        Mode::Light
    };
    eframe::run_native(
        "shadcn-egui gallery",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 900.0]),
            ..Default::default()
        },
        Box::new(move |_cc| {
            Ok(Box::new(GalleryApp {
                state: GalleryState::new(preset, mode),
            }))
        }),
    )
}
