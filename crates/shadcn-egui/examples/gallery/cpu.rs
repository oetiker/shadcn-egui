//! The gallery without a GPU: egui-winit for input, accessibility and the
//! clipboard, [`super::raster`] for the pixels, softbuffer to show them.
//!
//! It redraws only when egui asks for it (`repaint_delay`, or an input
//! event egui wants), re-rasterizes only the triangles that changed, and
//! sends only the changed rectangles to the window.
//!
//! It draws its own frames in `about_to_wait` and never asks winit for a
//! redraw, so a `RedrawRequested` always comes from the system: on X11 an
//! expose, after which the window has lost its pixels and gets the whole
//! frame. (winit merges expose and requested redraws into one event, so
//! the two could not be told apart otherwise.)

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::UserEvent;
use egui::{Color32, ViewportId, ViewportInfo};
use egui_winit::accesskit_winit;
use egui_winit::winit::application::ApplicationHandler;
use egui_winit::winit::event::WindowEvent;
use egui_winit::winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use egui_winit::winit::window::{Window, WindowId};
use shadcn_egui::reference::{gallery_ui, GalleryState};

use super::raster::{Canvas, IRect, Rasterizer};

/// Presents kept for `buffer.age()`: older back buffers get a full copy.
const DAMAGE_HISTORY: usize = 3;

struct Surface {
    window: Arc<Window>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
    egui_winit: egui_winit::State,
    info: ViewportInfo,
    canvas: Canvas,
    /// Rectangles redrawn in the last presents, newest first.
    history: Vec<Vec<IRect>>,
    shown: bool,
}

pub struct CpuApp {
    ctx: egui::Context,
    state: GalleryState,
    viewport: egui::ViewportBuilder,
    proxy: EventLoopProxy<UserEvent>,
    surface: Option<Surface>,
    raster: Rasterizer,
    /// When egui wants the next frame; `None` waits for input.
    next_frame: Option<Instant>,
    last_frame: Instant,
    frame_interval: Duration,
    /// Set by `SHADCN_EGUI_CPU_STATS=1`: prints frame and raster times.
    stats: bool,
    pub error: Option<String>,
}

impl CpuApp {
    pub fn new(
        ctx: egui::Context,
        state: GalleryState,
        viewport: egui::ViewportBuilder,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Self {
        // Requests from outside a pass (another thread, an image loader)
        // arrive as events; requests inside a pass come back as
        // `repaint_delay` and are handled after the frame.
        let callback_proxy = proxy.clone();
        ctx.set_request_repaint_callback(move |info| {
            // A delay past the end of time is no request.
            let Some(when) = Instant::now().checked_add(info.delay) else {
                return;
            };
            callback_proxy
                .send_event(UserEvent::RequestRepaint {
                    viewport_id: info.viewport_id,
                    when,
                    cumulative_pass_nr: info.current_cumulative_pass_nr,
                })
                .ok();
        });
        Self {
            ctx,
            state,
            viewport,
            proxy,
            surface: None,
            raster: Rasterizer::default(),
            next_frame: None,
            last_frame: Instant::now(),
            frame_interval: Duration::from_micros(16_667),
            stats: std::env::var_os("SHADCN_EGUI_CPU_STATS").is_some_and(|v| v == "1"),
            error: None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, what: &str, err: impl std::fmt::Display) {
        self.error = Some(format!("{what}: {err}"));
        event_loop.exit();
    }

    fn open_window(&mut self, event_loop: &ActiveEventLoop) -> Result<Surface, String> {
        // Hidden until the first frame is drawn; accesskit also needs the
        // adapter in place before the window shows.
        let builder = self.viewport.clone().with_visible(false);
        let window = egui_winit::create_window(&self.ctx, event_loop, &builder)
            .map_err(|e| format!("window: {e}"))?;
        let window = Arc::new(window);
        let context =
            softbuffer::Context::new(window.clone()).map_err(|e| format!("softbuffer: {e}"))?;
        let surface = softbuffer::Surface::new(&context, window.clone())
            .map_err(|e| format!("softbuffer: {e}"))?;
        let mut egui_winit = egui_winit::State::new(
            self.ctx.clone(),
            ViewportId::ROOT,
            event_loop,
            Some(window.scale_factor() as f32),
            event_loop.system_theme(),
            None,
        );
        egui_winit.init_accesskit(event_loop, &window, self.proxy.clone());
        if let Some(mhz) = window
            .current_monitor()
            .and_then(|m| m.refresh_rate_millihertz())
            .filter(|&mhz| mhz > 0)
        {
            self.frame_interval = Duration::from_secs_f64(1000.0 / f64::from(mhz));
        }
        let mut info = ViewportInfo::default();
        egui_winit::update_viewport_info(&mut info, &self.ctx, &window, true);
        Ok(Surface {
            window,
            surface,
            egui_winit,
            info,
            canvas: Canvas::new(0, 0),
            history: Vec::new(),
            shown: false,
        })
    }

    /// Runs one egui pass and shows what changed, or the whole frame after
    /// an `expose`.
    fn redraw(&mut self, event_loop: &ActiveEventLoop, expose: bool) {
        let Some(s) = self.surface.as_mut() else {
            return;
        };
        let started = Instant::now();
        self.last_frame = started;
        self.next_frame = None;

        egui_winit::update_viewport_info(&mut s.info, &self.ctx, &s.window, false);
        let mut raw_input = s.egui_winit.take_egui_input(&s.window);
        raw_input.viewports = std::iter::once((ViewportId::ROOT, s.info.clone())).collect();
        let state = &mut self.state;
        let mut output = self.ctx.run_ui(raw_input, |ui| gallery_ui(ui, state));

        if let Some(vp) = output.viewport_output.remove(&ViewportId::ROOT) {
            let mut actions = Vec::new();
            egui_winit::process_viewport_commands(
                &self.ctx,
                &mut s.info,
                vp.commands,
                &s.window,
                &mut actions,
            );
            if vp.repaint_delay != Duration::MAX {
                self.next_frame = Some(started + vp.repaint_delay.max(self.frame_interval));
            }
        }
        s.egui_winit
            .handle_platform_output(&s.window, output.platform_output);
        if s.info.close_requested() {
            event_loop.exit();
            return;
        }

        let size = s.window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            self.raster.free_textures(&output.textures_delta);
            return; // minimized
        };
        if (s.canvas.width, s.canvas.height) != (w.get() as usize, h.get() as usize) {
            if let Err(e) = s.surface.resize(w, h) {
                self.fail(event_loop, "softbuffer resize", e);
                return;
            }
            s.canvas = Canvas::new(w.get() as usize, h.get() as usize);
            s.history.clear();
        }

        let ppp = output.pixels_per_point;
        let primitives = self.ctx.tessellate(output.shapes, ppp);
        let tessellated = Instant::now();
        self.raster.set_textures(&output.textures_delta);
        let damage = self
            .raster
            .render(&mut s.canvas, &primitives, ppp, clear_color());
        self.raster.free_textures(&output.textures_delta);
        let rastered = Instant::now();

        // Pixels presented while the window is still hidden are lost; the
        // expose that follows `set_visible` presents them again.
        if !damage.is_empty() || expose || !s.shown {
            if let Err(e) = present(s, damage.clone(), expose) {
                self.fail(event_loop, "softbuffer present", e);
                return;
            }
        }
        if !s.shown {
            s.shown = true;
            s.window.set_visible(true);
        }
        if self.stats && !damage.is_empty() {
            let px: i64 = damage
                .iter()
                .map(|r| i64::from(r.x1 - r.x0) * i64::from(r.y1 - r.y0))
                .sum();
            eprintln!(
                "cpu frame: ppp {ppp} pass+tessellate {:.1} ms, raster {:.1} ms ({px} px in {} rects), present {:.1} ms",
                (tessellated - started).as_secs_f64() * 1e3,
                (rastered - tessellated).as_secs_f64() * 1e3,
                damage.len(),
                rastered.elapsed().as_secs_f64() * 1e3,
            );
        }
    }

    /// Draws the frame when it is due and sets when the loop wakes next.
    fn schedule(&mut self, event_loop: &ActiveEventLoop) {
        if self.next_frame.is_some_and(|at| at <= Instant::now()) {
            self.redraw(event_loop, false);
        }
        match self.next_frame {
            Some(at) => event_loop.set_control_flow(ControlFlow::WaitUntil(at)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }

    /// Draws a frame at the end of this round of events.
    fn frame_now(&mut self) {
        self.next_frame = Some(Instant::now());
    }

    fn want_frame_now(&mut self) {
        // Input is answered at once, but no faster than the display shows
        // frames, so a stream of mouse moves does not pin a core.
        let at = (self.last_frame + self.frame_interval).max(Instant::now());
        self.next_frame = Some(self.next_frame.map_or(at, |t| t.min(at)));
    }
}

/// Copies what changed since this back buffer was last shown and presents
/// those rectangles only (on X11 with shared memory only they go to the
/// server), or the whole frame when `all`.
fn present(
    s: &mut Surface,
    damage: Vec<IRect>,
    all: bool,
) -> Result<(), softbuffer::SoftBufferError> {
    let full = s.canvas.rect();
    s.history.insert(0, damage);
    s.history.truncate(DAMAGE_HISTORY);
    let mut buffer = s.surface.buffer_mut()?;
    let age = usize::from(buffer.age());
    let copy: Vec<IRect> = if age == 0 || age > s.history.len() {
        vec![full]
    } else {
        s.history[..age].concat()
    };
    let width = s.canvas.width;
    for r in &copy {
        for y in r.y0..r.y1 {
            let row = y as usize * width;
            let span = row + r.x0 as usize..row + r.x1 as usize;
            buffer[span.clone()].copy_from_slice(&s.canvas.pixels[span]);
        }
    }
    let rects: Vec<softbuffer::Rect> = s.history[0]
        .iter()
        .chain(if age == 0 || all {
            std::slice::from_ref(&full)
        } else {
            &[]
        })
        .filter_map(|r| {
            Some(softbuffer::Rect {
                x: r.x0 as u32,
                y: r.y0 as u32,
                width: NonZeroU32::new((r.x1 - r.x0) as u32)?,
                height: NonZeroU32::new((r.y1 - r.y0) as u32)?,
            })
        })
        .collect();
    if rects.is_empty() {
        buffer.present()
    } else {
        buffer.present_with_damage(&rects)
    }
}

/// eframe's default clear color, as its wgpu path leaves it in an opaque
/// window.
fn clear_color() -> u32 {
    let [r, g, b, _] = Color32::from_rgba_unmultiplied(12, 12, 12, 180).to_array();
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

impl ApplicationHandler<UserEvent> for CpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.surface.is_some() {
            return;
        }
        match self.open_window(event_loop) {
            Ok(s) => {
                self.surface = Some(s);
                self.frame_now();
            }
            Err(e) => self.fail(event_loop, "cpu renderer", e),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        // After a failed wgpu start, the window eframe opened on this loop
        // still reports its `Destroyed` here; it is not ours to act on.
        let Some(s) = self.surface.as_mut().filter(|s| s.window.id() == id) else {
            return;
        };
        match event {
            // Only the system asks for this (see the module comment).
            WindowEvent::RedrawRequested => self.redraw(event_loop, true),
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                event_loop.exit();
            }
            event => {
                let resized = matches!(
                    event,
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
                );
                let response = s.egui_winit.on_window_event(&s.window, &event);
                if resized {
                    // A resize shows a frame at the new size at once.
                    self.frame_now();
                } else if response.repaint {
                    self.want_frame_now();
                }
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::RequestRepaint {
                when,
                cumulative_pass_nr,
                ..
            } => {
                // A request from a pass that already ran is stale.
                if self.ctx.cumulative_pass_nr_for(ViewportId::ROOT) == cumulative_pass_nr {
                    self.next_frame = Some(self.next_frame.map_or(when, |t| t.min(when)));
                }
            }
            UserEvent::AccessKitActionRequest(request) => {
                let Some(s) = self.surface.as_mut() else {
                    return;
                };
                match request.window_event {
                    accesskit_winit::WindowEvent::InitialTreeRequested => {
                        self.ctx.enable_accesskit();
                        self.frame_now();
                    }
                    accesskit_winit::WindowEvent::ActionRequested(action) => {
                        s.egui_winit.on_accesskit_action_request(action);
                        self.want_frame_now();
                    }
                    accesskit_winit::WindowEvent::AccessibilityDeactivated => {
                        self.ctx.disable_accesskit();
                    }
                }
            }
        }
    }

    // Every round of events ends here: one frame answers all of them.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.schedule(event_loop);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // The window and its surface go while the event loop still runs.
        self.surface = None;
    }
}
