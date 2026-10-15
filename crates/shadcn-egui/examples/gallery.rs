//! Every component under a live preset picker:
//! `cargo run --example gallery --features fonts-all,icons-all [-- <preset code> [dark|light]]`.
//!
//! The same example runs in the browser: `scripts/build-web-gallery.sh`
//! builds it, and the page takes `?preset=<code>&mode=dark`.

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

/// The starting state from a preset code and a mode, as the desktop
/// arguments or the page's query string give them, plus a warning for the
/// user when the code does not decode.
fn initial_state(preset: Option<&str>, mode: Option<&str>) -> (GalleryState, Option<String>) {
    let mut warning = None;
    let preset = match preset {
        Some(code) => Preset::from_code(code).unwrap_or_else(|e| {
            warning = Some(format!("gallery: {code}: {e}; using the default preset"));
            Preset::default()
        }),
        None => Preset::default(),
    };
    let mode = if mode == Some("dark") {
        Mode::Dark
    } else {
        Mode::Light
    };
    (GalleryState::new(preset, mode), warning)
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (state, warning) = initial_state(
        args.get(1).map(String::as_str),
        args.get(2).map(String::as_str),
    );
    if let Some(warning) = warning {
        eprintln!("{warning}");
    }
    eframe::run_native(
        "shadcn-egui gallery",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 900.0]),
            ..Default::default()
        },
        Box::new(move |_cc| Ok(Box::new(GalleryApp { state }))),
    )
}

/// The page's loader script (`web/index.html`) defines both functions:
/// one removes the loading card, the other shows the message on it.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = galleryStarted)]
    fn gallery_started();
    #[wasm_bindgen(js_namespace = window, js_name = galleryFailed)]
    fn gallery_failed(message: &str);
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::JsCast as _;

    eframe::WebLogger::init(log::LevelFilter::Warn).ok();
    let window = web_sys::window().expect("a browser window");
    let query = window.location().search().unwrap_or_default();
    let params = web_sys::UrlSearchParams::new_with_str(&query).expect("a query string");
    let (state, warning) = initial_state(
        params.get("preset").as_deref(),
        params.get("mode").as_deref(),
    );
    if let Some(warning) = warning {
        log::warn!("{warning}");
    }
    let canvas = window
        .document()
        .and_then(|d| d.get_element_by_id("gallery"))
        .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        .expect("<canvas id=\"gallery\">");
    wasm_bindgen_futures::spawn_local(async move {
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |_cc| Ok(Box::new(GalleryApp { state }))),
            )
            .await;
        match started {
            Ok(()) => gallery_started(),
            Err(e) => gallery_failed(&e.as_string().unwrap_or_else(|| format!("{e:?}"))),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_given_is_the_default_preset_in_light_mode() {
        let (state, warning) = initial_state(None, None);
        assert_eq!(state.preset, Preset::default());
        assert_eq!(state.mode, Mode::Light);
        assert_eq!(warning, None);
    }

    #[test]
    fn a_valid_code_and_dark_apply() {
        // The code of the default preset with only the style set to Vega.
        let code = "bIkeymG";
        let (state, warning) = initial_state(Some(code), Some("dark"));
        let expected = Preset {
            style: shadcn_egui::preset::StyleName::Vega,
            ..Preset::default()
        };
        assert_eq!(Preset::from_code(code).expect("decodes"), expected);
        assert_eq!(expected.to_code().as_deref(), Some(code));
        assert_ne!(state.preset, Preset::default());
        assert_eq!(state.preset, expected);
        assert_eq!(state.mode, Mode::Dark);
        assert_eq!(warning, None);
    }

    #[test]
    fn a_bad_code_warns_and_falls_back_to_the_default() {
        let (state, warning) = initial_state(Some("c0"), None);
        assert_eq!(state.preset, Preset::default());
        let warning = warning.expect("a warning");
        assert!(warning.contains("c0"), "{warning}");
    }

    #[test]
    fn any_mode_other_than_dark_is_light() {
        for mode in ["light", "DARK", ""] {
            assert_eq!(
                initial_state(None, Some(mode)).0.mode,
                Mode::Light,
                "{mode}"
            );
        }
    }
}
