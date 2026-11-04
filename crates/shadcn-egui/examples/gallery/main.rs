//! Every component under a live preset picker:
//! `cargo run --example gallery --features fonts-all,icons-all [-- [--cpu] <preset code> [dark|light]]`.
//!
//! On the desktop it draws with wgpu, as eframe does by default. `--cpu`
//! draws on the CPU instead (egui-winit, a rasterizer of egui's meshes and
//! softbuffer, in `cpu.rs` and `raster.rs`), and the gallery switches to the
//! CPU by itself when wgpu finds no adapter, or only a software one such
//! as llvmpipe (`SHADCN_EGUI_SOFTWARE_GPU=1` lets wgpu use that one).
//! `SHADCN_EGUI_CPU_STATS=1` prints the frame times of either path.
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

/// What eframe logs as it ends its loop on an error; it returns the error
/// only from `run_native`, which the gallery cannot use (see `main`). The
/// text is eframe's wording, not its API: the test
/// `eframe_still_logs_the_exit_error_the_gallery_reads` finds it in the
/// eframe source `Cargo.lock` pins.
#[cfg(not(target_arch = "wasm32"))]
const EFRAME_EXIT: &str = "Exiting because of error: ";

/// The process's logger: it keeps the error eframe ended its loop with
/// (the last one if several) and hands every other record to `inner`, the
/// logger the app would have set anyway. An app with its own logger puts
/// that one in `inner` instead of [`Stderr`].
#[cfg(not(target_arch = "wasm32"))]
struct EframeError {
    inner: &'static dyn log::Log,
    cause: std::sync::Mutex<Option<String>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl EframeError {
    /// Sets the logger for the whole process; `max_level` is what `inner`
    /// wants to see. `None` when another logger is set already: eframe's
    /// error is then lost.
    fn install(inner: &'static dyn log::Log, max_level: log::LevelFilter) -> Option<&'static Self> {
        let logger: &'static Self = Box::leak(Box::new(Self::new(inner)));
        log::set_logger(logger).ok()?;
        log::set_max_level(Self::global_level(max_level));
        Some(logger)
    }

    /// The process's level: what `inner` wants, and eframe's error always.
    fn global_level(max_level: log::LevelFilter) -> log::LevelFilter {
        max_level.max(log::LevelFilter::Error)
    }

    fn new(inner: &'static dyn log::Log) -> Self {
        Self {
            inner,
            cause: std::sync::Mutex::new(None),
        }
    }

    fn take(&self) -> Option<String> {
        self.cause.lock().map_or(None, |mut e| e.take())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl log::Log for EframeError {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() == log::Level::Error || self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if record.level() == log::Level::Error {
            let message = record.args().to_string();
            if let Some(cause) = message.strip_prefix(EFRAME_EXIT) {
                // The gallery reports it itself, after the loop.
                if let Ok(mut slot) = self.cause.lock() {
                    *slot = Some(cause.to_owned());
                }
                return;
            }
        }
        if self.inner.enabled(record.metadata()) {
            self.inner.log(record);
        }
    }

    fn flush(&self) {
        self.inner.flush();
    }
}

/// Prints warnings and errors to stderr: the crate's own (a preset value
/// this build cannot show), egui's, eframe's and wgpu's. winit's only from
/// errors up: it warns on every start on many X11 desktops (no XSETTINGS
/// daemon, a screen of 0 mm), which the user cannot act on.
#[cfg(not(target_arch = "wasm32"))]
struct Stderr;

#[cfg(not(target_arch = "wasm32"))]
impl log::Log for Stderr {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let target = metadata.target();
        let winit = target == "winit" || target.starts_with("winit::");
        metadata.level()
            <= if winit {
                log::Level::Error
            } else {
                log::Level::Warn
            }
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            eprintln!("[{} {}] {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

/// `SHADCN_EGUI_SOFTWARE_GPU=1` lets wgpu draw on a software adapter.
#[cfg(not(target_arch = "wasm32"))]
const ALLOW_SOFTWARE: &str = "SHADCN_EGUI_SOFTWARE_GPU";

/// What the gallery needs to know of a wgpu adapter to choose one.
#[cfg(not(target_arch = "wasm32"))]
struct Adapter {
    device_type: eframe::wgpu::DeviceType,
    name: String,
    /// It can draw to the window's surface.
    fits: bool,
}

/// The adapter wgpu would choose for `power` (its own order of device
/// types, the first of a kind), as an index into `adapters`. A software
/// adapter (`DeviceType::Cpu`: llvmpipe, lavapipe, WARP) is refused unless
/// `allow_software`: the gallery's own CPU renderer then draws instead,
/// at a fraction of the CPU time.
#[cfg(not(target_arch = "wasm32"))]
fn choose_adapter(
    adapters: &[Adapter],
    power: eframe::wgpu::PowerPreference,
    allow_software: bool,
) -> Result<usize, String> {
    use eframe::wgpu::{DeviceType, PowerPreference};
    // wgpu-core's order in `request_adapter`.
    let order = |t: DeviceType| match (power, t) {
        (PowerPreference::None, _) => 0,
        (PowerPreference::LowPower, DeviceType::IntegratedGpu) => 1,
        (PowerPreference::LowPower, DeviceType::DiscreteGpu) => 2,
        (_, DeviceType::DiscreteGpu) => 1,
        (_, DeviceType::IntegratedGpu) => 2,
        (_, DeviceType::Other) => 3,
        (_, DeviceType::VirtualGpu) => 4,
        (_, DeviceType::Cpu) => 5,
    };
    let usable = |a: &Adapter| a.fits && (allow_software || a.device_type != DeviceType::Cpu);
    let best = adapters
        .iter()
        .enumerate()
        .filter(|(_, a)| usable(a))
        .min_by_key(|(_, a)| order(a.device_type));
    if let Some((i, _)) = best {
        return Ok(i);
    }
    Err(match adapters.iter().find(|a| a.fits) {
        Some(software) => format!(
            "only a software adapter, {}. Set {ALLOW_SOFTWARE}=1 to use it.",
            software.name
        ),
        None if adapters.is_empty() => "no wgpu adapter".to_owned(),
        None => "no wgpu adapter can draw to the window".to_owned(),
    })
}

/// eframe's wgpu set-up with the gallery's choice of adapter.
#[cfg(not(target_arch = "wasm32"))]
fn wgpu_options(allow_software: bool) -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
    use eframe::wgpu::PowerPreference;
    let power = PowerPreference::from_env().unwrap_or(PowerPreference::HighPerformance);
    let setup = WgpuSetupCreateNew {
        power_preference: power,
        native_adapter_selector: Some(std::sync::Arc::new(move |adapters, surface| {
            let found: Vec<Adapter> = adapters
                .iter()
                .map(|a| {
                    let info = a.get_info();
                    Adapter {
                        device_type: info.device_type,
                        name: info.name,
                        fits: surface.is_none_or(|s| a.is_surface_supported(s)),
                    }
                })
                .collect();
            choose_adapter(&found, power, allow_software).map(|i| adapters[i].clone())
        })),
        ..WgpuSetupCreateNew::without_display_handle()
    };
    WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    }
}

/// What follows eframe's loop: `Ok(None)` ends the gallery, `Ok(Some(..))`
/// is the note to print before the CPU takes over, `Err` is the error
/// wgpu failed with after it had started.
#[cfg(not(target_arch = "wasm32"))]
fn after_eframe(started: bool, error: Option<String>) -> Result<Option<String>, String> {
    match (started, error) {
        (true, None) => Ok(None),
        (true, Some(e)) => Err(e),
        (false, Some(e)) => Ok(Some(format!(
            "gallery: drawing on the CPU, as wgpu did not start: {e}"
        ))),
        (false, None) => Ok(Some(
            "gallery: drawing on the CPU, as wgpu did not start".to_owned(),
        )),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    if let Err(e) = run() {
        eprintln!("gallery: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use egui_winit::winit::event_loop::EventLoop;
    use egui_winit::winit::platform::run_on_demand::EventLoopExtRunOnDemand as _;

    // Before anything logs: the preset warnings come from initial_state.
    let logger = EframeError::install(&Stderr, log::LevelFilter::Warn);
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cpu, preset, mode) = parse_args(&args);
    let (state, warning) = initial_state(preset, mode);
    if let Some(warning) = warning {
        eprintln!("{warning}");
    }
    let mut state = Some(state);

    // winit allows one event loop per process: both renderers run on it,
    // the CPU one after a failed wgpu start.
    let mut event_loop = EventLoop::<eframe::UserEvent>::with_user_event().build()?;

    if !cpu {
        // eframe creates the app only once wgpu has an adapter, a device
        // and a surface for the window; any failure there ends the loop
        // before that, whatever the cause.
        let wgpu_started = std::rc::Rc::new(std::cell::Cell::new(false));
        let started = wgpu_started.clone();
        let state = state.take();
        let mut app = eframe::create_native(
            TITLE,
            eframe::NativeOptions {
                viewport: viewport(),
                wgpu_options: wgpu_options(
                    std::env::var_os(ALLOW_SOFTWARE).is_some_and(|v| v == "1"),
                ),
                // eframe 0.37 asks the system for a font for any char the
                // theme's fonts lack; the CPU path, the browser build and
                // the snapshot tests draw with the theme's fonts alone, so
                // this one does too, and all draw the same picture.
                system_font_fallback: false,
                ..Default::default()
            },
            Box::new(move |cc| {
                started.set(true);
                let state = state.unwrap_or_else(|| initial_state(preset, mode).0);
                Ok(Box::new(GalleryApp::new(&cc.egui_ctx, state)))
            }),
            &event_loop,
        );
        event_loop.run_app_on_demand(&mut app)?;
        drop(app);
        let error = logger.and_then(EframeError::take);
        match after_eframe(wgpu_started.get(), error).map_err(|e| format!("wgpu failed: {e}"))? {
            None => return Ok(()),
            Some(note) => eprintln!("{note}"),
        }
    }

    let ctx = egui::Context::default();
    // A failed wgpu start used the state up; its warning is out already.
    let state = state.unwrap_or_else(|| initial_state(preset, mode).0);
    state.preset.apply(&ctx);
    ctx.set_theme(state.mode);
    let mut app = cpu::CpuApp::new(ctx, state, viewport(), event_loop.create_proxy());
    event_loop.run_app_on_demand(&mut app)?;
    match app.error.take() {
        Some(e) => Err(format!("cpu renderer failed: {e}").into()),
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
    fn eframe_s_error_ends_the_gallery_or_names_the_fallback_s_cause() {
        assert_eq!(after_eframe(true, None), Ok(None));
        assert_eq!(
            after_eframe(true, Some("device lost".into())),
            Err("device lost".to_owned())
        );
        assert_eq!(
            after_eframe(false, Some("no adapter".into())),
            Ok(Some(
                "gallery: drawing on the CPU, as wgpu did not start: no adapter".to_owned()
            ))
        );
        assert_eq!(
            after_eframe(false, None),
            Ok(Some(
                "gallery: drawing on the CPU, as wgpu did not start".to_owned()
            ))
        );
    }

    /// Counts the records it is handed, as an app's own logger would get
    /// them.
    struct Counter(std::sync::atomic::AtomicUsize);

    impl log::Log for Counter {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= log::Level::Warn
        }
        fn log(&self, _: &log::Record) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        fn flush(&self) {}
    }

    #[test]
    fn the_logger_keeps_eframe_s_exit_error_and_passes_the_rest_on() {
        use log::Log as _;
        static INNER: Counter = Counter(std::sync::atomic::AtomicUsize::new(0));
        let passed = || INNER.0.load(std::sync::atomic::Ordering::Relaxed);
        let logger = EframeError::new(&INNER);
        let record = |level, message: &str| {
            logger.log(
                &log::Record::builder()
                    .level(level)
                    .args(format_args!("{message}"))
                    .build(),
            );
        };
        record(log::Level::Error, "something else");
        record(log::Level::Warn, "Exiting because of error: a warning");
        record(log::Level::Warn, "Missing texture: Managed(9)");
        record(log::Level::Info, "not for the inner logger");
        assert_eq!(logger.take(), None);
        assert_eq!(passed(), 3);
        record(log::Level::Error, "Exiting because of error: no adapter");
        assert_eq!(passed(), 3, "the gallery reports the cause itself");
        assert_eq!(logger.take().as_deref(), Some("no adapter"));
        assert_eq!(logger.take(), None);
    }

    /// The only test in this binary that sets the process's logger.
    #[test]
    fn the_installed_logger_lets_warnings_through_to_the_inner_one() {
        use std::sync::atomic::{AtomicBool, Ordering};
        /// Notes the one warning this test logs; other tests may log too.
        struct Seen(AtomicBool);
        impl log::Log for Seen {
            fn enabled(&self, metadata: &log::Metadata) -> bool {
                metadata.level() <= log::Level::Warn
            }
            fn log(&self, record: &log::Record) {
                if record.args().to_string() == "a preset value this build cannot show" {
                    self.0.store(true, Ordering::Relaxed);
                }
            }
            fn flush(&self) {}
        }
        static SEEN: Seen = Seen(AtomicBool::new(false));
        let logger = EframeError::install(&SEEN, log::LevelFilter::Warn).expect("the first logger");
        assert_eq!(log::max_level(), log::LevelFilter::Warn);
        log::warn!(target: "shadcn_egui", "a preset value this build cannot show");
        assert!(
            SEEN.0.load(Ordering::Relaxed),
            "the warning did not reach the inner logger"
        );
        log::error!("{EFRAME_EXIT}no adapter");
        assert_eq!(logger.take().as_deref(), Some("no adapter"));

        // eframe's error is kept even when the inner logger wants nothing.
        assert_eq!(
            EframeError::global_level(log::LevelFilter::Off),
            log::LevelFilter::Error
        );
        assert_eq!(
            EframeError::global_level(log::LevelFilter::Info),
            log::LevelFilter::Info
        );
    }

    #[test]
    fn stderr_prints_warnings_but_only_winit_s_errors() {
        use log::Log as _;
        let enabled = |level, target| {
            Stderr.enabled(&log::Metadata::builder().level(level).target(target).build())
        };
        for target in [
            "shadcn_egui::preset",
            "egui_wgpu",
            "eframe",
            "gallery",
            "winitish",
        ] {
            assert!(enabled(log::Level::Warn, target), "{target}");
            assert!(!enabled(log::Level::Info, target), "{target}");
        }
        for target in ["winit", "winit::platform_impl::linux::x11::xdisplay"] {
            assert!(!enabled(log::Level::Warn, target), "{target}");
            assert!(enabled(log::Level::Error, target), "{target}");
        }
    }

    /// `EFRAME_EXIT` is eframe's log text, not its API: this fails when the
    /// eframe that `Cargo.lock` pins no longer logs it as an error.
    #[test]
    fn eframe_still_logs_the_exit_error_the_gallery_reads() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let lock = std::fs::read_to_string(root.join("Cargo.lock")).expect("Cargo.lock");
        let version = lock
            .split("[[package]]")
            .find(|p| p.contains("\nname = \"eframe\"\n"))
            .and_then(|p| p.lines().find_map(|l| l.strip_prefix("version = ")))
            .expect("eframe in Cargo.lock")
            .trim_matches('"');
        let cargo_home = std::env::var_os("CARGO_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".cargo")))
            .expect("CARGO_HOME or HOME");
        let registry = cargo_home.join("registry/src");
        let wanted = format!("log::error!(\"{EFRAME_EXIT}{{");
        let package = format!("eframe-{version}/src");
        let sources: Vec<_> = std::fs::read_dir(&registry)
            .map(|indexes| {
                indexes
                    .filter_map(|index| Some(index.ok()?.path().join(&package)))
                    .filter(|src| src.is_dir())
                    .collect()
            })
            .unwrap_or_default();
        assert!(
            !sources.is_empty(),
            "the eframe {version} source was not found: looked for `{package}` in every \
             index under {} (vendored sources or a source replacement keep it elsewhere)",
            registry.display()
        );
        assert!(
            sources.iter().any(|src| contains_text(src, &wanted)),
            "eframe {version} no longer logs `{wanted}`: searched {sources:?}"
        );
    }

    fn contains_text(dir: &std::path::Path, text: &str) -> bool {
        std::fs::read_dir(dir)
            .expect("a source directory")
            .any(|entry| {
                let path = entry.expect("a directory entry").path();
                if path.is_dir() {
                    contains_text(&path, text)
                } else {
                    path.extension().is_some_and(|e| e == "rs")
                        && std::fs::read_to_string(&path).is_ok_and(|s| s.contains(text))
                }
            })
    }

    #[test]
    fn a_software_adapter_is_refused_unless_allowed() {
        use eframe::wgpu::{DeviceType, PowerPreference};
        let high = PowerPreference::HighPerformance;
        let a = |device_type, name: &str, fits| Adapter {
            device_type,
            name: name.to_owned(),
            fits,
        };
        let llvmpipe = a(DeviceType::Cpu, "llvmpipe", true);
        let gpu = a(DeviceType::DiscreteGpu, "gpu", true);
        let igpu = a(DeviceType::IntegratedGpu, "igpu", true);
        let gl = a(DeviceType::Other, "gl", true);

        // Software only: the gallery draws on the CPU, and says why.
        let only =
            choose_adapter(std::slice::from_ref(&llvmpipe), high, false).expect_err("refused");
        assert!(only.contains("software adapter, llvmpipe"), "{only}");
        assert!(
            only.ends_with("Set SHADCN_EGUI_SOFTWARE_GPU=1 to use it."),
            "{only}"
        );
        assert_eq!(
            choose_adapter(std::slice::from_ref(&llvmpipe), high, true),
            Ok(0)
        );

        // A real GPU is used, software or not beside it, in wgpu's order.
        let list = [llvmpipe, igpu, gpu, gl];
        assert_eq!(choose_adapter(&list, high, false), Ok(2));
        assert_eq!(choose_adapter(&list, high, true), Ok(2));
        assert_eq!(
            choose_adapter(&list, PowerPreference::LowPower, false),
            Ok(1)
        );
        assert_eq!(
            choose_adapter(&[a(DeviceType::Other, "gl", true)], high, false),
            Ok(0)
        );
        // `PowerPreference::None` keeps the listed order, so llvmpipe comes
        // first, yet a GPU beside it is used.
        assert_eq!(choose_adapter(&list, PowerPreference::None, false), Ok(1));
        assert_eq!(choose_adapter(&list, PowerPreference::None, true), Ok(0));

        // An adapter that cannot draw to the window does not count.
        let unfit = [
            a(DeviceType::DiscreteGpu, "gpu", false),
            a(DeviceType::Cpu, "llvmpipe", true),
        ];
        assert!(choose_adapter(&unfit, high, false).is_err());
        assert_eq!(
            choose_adapter(&[], high, false),
            Err("no wgpu adapter".to_owned())
        );
        assert_eq!(
            choose_adapter(&unfit[..1], high, false),
            Err("no wgpu adapter can draw to the window".to_owned())
        );
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
