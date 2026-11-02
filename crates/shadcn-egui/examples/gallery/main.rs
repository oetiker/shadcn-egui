//! Every component under a live preset picker:
//! `cargo run --example gallery --features fonts-all,icons-all [-- [--cpu] <preset code> [dark|light]]`.
//!
//! On the desktop it draws with wgpu, as eframe does by default. `--cpu`
//! draws on the CPU instead (egui-winit, a rasterizer of egui's meshes and
//! softbuffer, in `cpu.rs` and `raster.rs`), and the gallery switches to the
//! CPU by itself when wgpu finds no adapter. `SHADCN_EGUI_CPU_STATS=1`
//! prints the frame times of either path.
//!
//! The same example runs in the browser: `scripts/build-web-gallery.sh`
//! builds it, and the page takes `?preset=<code>&mode=dark`; it always
//! draws with WebGPU or WebGL.

#[cfg(not(target_arch = "wasm32"))]
mod cpu;
#[cfg(not(target_arch = "wasm32"))]
mod raster;

use shadcn_egui::reference::{gallery_ui, GalleryState};
use shadcn_egui::Preset;

struct GalleryApp {
    state: GalleryState,
    /// `SHADCN_EGUI_CPU_STATS=1` on the desktop: prints the wgpu path's
    /// frame times too, for comparison with the CPU path.
    stats: bool,
}

impl GalleryApp {
    /// The app for `state`, its preset and mode applied before the first
    /// pass, so the first frame is in the preset's fonts. `gallery_ui`
    /// applies them again in its first pass, which then changes nothing.
    fn new(ctx: &egui::Context, state: GalleryState) -> Self {
        state.preset.apply(ctx);
        ctx.set_theme(state.mode);
        let stats = cfg!(not(target_arch = "wasm32"))
            && std::env::var_os("SHADCN_EGUI_CPU_STATS").is_some_and(|v| v == "1");
        Self { state, stats }
    }
}

impl eframe::App for GalleryApp {
    // gallery_ui applies the preset itself, only when a picker changed it.
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if let Some(seconds) = frame.info().cpu_usage.filter(|_| self.stats) {
            // eframe's own measure: pass, tessellation and the wgpu
            // submission, without the wait for the display.
            eprintln!("wgpu frame (previous): {:.1} ms", seconds * 1e3);
        }
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
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    (GalleryState::new(preset, mode), warning)
}

/// The window both renderers open.
#[cfg(not(target_arch = "wasm32"))]
fn viewport() -> egui::ViewportBuilder {
    egui::ViewportBuilder::default()
        .with_title(TITLE)
        .with_app_id(TITLE)
        .with_inner_size([1280.0, 900.0])
}

#[cfg(not(target_arch = "wasm32"))]
const TITLE: &str = "shadcn-egui gallery";

/// `--cpu` anywhere, the preset code and the mode in that order.
#[cfg(not(target_arch = "wasm32"))]
fn parse_args(args: &[String]) -> (bool, Option<&str>, Option<&str>) {
    let cpu = args.iter().any(|a| a == "--cpu");
    let mut rest = args.iter().filter(|a| *a != "--cpu").map(String::as_str);
    (cpu, rest.next(), rest.next())
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use egui_winit::winit::event_loop::EventLoop;
    use egui_winit::winit::platform::run_on_demand::EventLoopExtRunOnDemand as _;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cpu, preset, mode) = parse_args(&args);
    let state = || {
        let (state, warning) = initial_state(preset, mode);
        if let Some(warning) = warning {
            eprintln!("{warning}");
        }
        state
    };

    // winit allows one event loop per process: both renderers run on it,
    // the CPU one after a failed wgpu start.
    let mut event_loop = EventLoop::<eframe::UserEvent>::with_user_event().build()?;

    if !cpu {
        // eframe creates the app only once wgpu has an adapter, a device
        // and a surface for the window; any failure there ends the loop
        // before that, whatever the cause.
        let wgpu_started = std::rc::Rc::new(std::cell::Cell::new(false));
        let started = wgpu_started.clone();
        let state = state();
        let mut app = eframe::create_native(
            TITLE,
            eframe::NativeOptions {
                viewport: viewport(),
                ..Default::default()
            },
            Box::new(move |cc| {
                started.set(true);
                Ok(Box::new(GalleryApp::new(&cc.egui_ctx, state)))
            }),
            &event_loop,
        );
        event_loop.run_app_on_demand(&mut app)?;
        drop(app);
        if wgpu_started.get() {
            return Ok(());
        }
        eprintln!("gallery: wgpu did not start (no usable GPU adapter); drawing on the CPU");
    }

    let ctx = egui::Context::default();
    let state = state();
    state.preset.apply(&ctx);
    ctx.set_theme(state.mode);
    let mut app = cpu::CpuApp::new(ctx, state, viewport(), event_loop.create_proxy());
    event_loop.run_app_on_demand(&mut app)?;
    match app.error.take() {
        Some(e) => Err(e.into()),
        None => Ok(()),
    }
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
                Box::new(move |cc| Ok(Box::new(GalleryApp::new(&cc.egui_ctx, state)))),
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
        assert_eq!(state.mode, egui::Theme::Light);
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
        assert_eq!(state.mode, egui::Theme::Dark);
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
    fn cpu_is_a_flag_anywhere_and_the_rest_stays_in_order() {
        let args = |a: &[&str]| a.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let none = args(&[]);
        assert_eq!(parse_args(&none), (false, None, None));
        let first = args(&["--cpu", "bIkeymG", "dark"]);
        assert_eq!(parse_args(&first), (true, Some("bIkeymG"), Some("dark")));
        let middle = args(&["bIkeymG", "--cpu", "dark"]);
        assert_eq!(parse_args(&middle), (true, Some("bIkeymG"), Some("dark")));
        let plain = args(&["bIkeymG"]);
        assert_eq!(parse_args(&plain), (false, Some("bIkeymG"), None));
    }

    #[test]
    fn any_mode_other_than_dark_is_light() {
        for mode in ["light", "DARK", ""] {
            assert_eq!(
                initial_state(None, Some(mode)).0.mode,
                egui::Theme::Light,
                "{mode}"
            );
        }
    }
}
