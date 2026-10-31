//! shadcn Spinner, ported from shadcn-svelte `ui/spinner/spinner.svelte` at
//! `tools/stylegen/PIN`. Every value comes from the style's spinner recipe
//! (`size-4 animate-spin`); the icon is the theme's `loader` slot, upstream's
//! `Loader2Icon` in lucide and each icon library's own loader.
//!
//! Upstream turns the icon with a plain `animate-spin`, and the browser keeps
//! it turning under reduced motion. The port stops it there instead: D-88 (c)
//! makes every animation of the crate honour `style.animation_time == 0` as
//! reduced motion, so a spinner stands still at its start angle and asks for
//! no repaint.
//!
//! In a button or badge the spinner takes the place of an icon:
//! `.icon(Glyph::Spinner)`.

use crate::components::icon::paint_icon_rotated;
#[cfg(doc)]
use crate::components::icon::Glyph;
use crate::components::shared::widget_info_with_role;
use crate::style::{pick, Easing, Recipe, State};
use crate::Theme;
use egui::accesskit::{Live, Role};
use egui::{Color32, Rect, Response, Sense, Ui, UiBuilder, Vec2, Widget, WidgetInfo, WidgetType};

/// A turning loader icon: `ui.add(Spinner::new())`, or [`spinner`].
///
/// It is the style's spinner size (16 px), draws in the `Ui`'s text colour
/// (`currentColor`), and turns once per the style's duration while it is
/// visible. Screen readers hear a status named "Loading"; `.alt_text(..)`
/// names it otherwise.
///
/// ```
/// use shadcn_egui::components::spinner::{spinner, Spinner};
///
/// fn busy(ui: &mut egui::Ui, side: f32) {
///     spinner(ui);
///     ui.add(Spinner::new().size(side).alt_text("Saving"));
/// }
/// ```
#[derive(Clone, Debug)]
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Spinner {
    size: Option<f32>,
    color: Option<Color32>,
    alt_text: String,
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            size: None,
            color: None,
            alt_text: LOADING.to_owned(),
        }
    }
    /// The side (`size-*`); the style's spinner size by default.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }
    /// The colour; the `Ui`'s text colour by default.
    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.color = Some(color.into());
        self
    }
    /// The name screen readers hear; "Loading" by default.
    #[doc(alias = "aria-label")]
    pub fn alt_text(mut self, text: impl Into<String>) -> Self {
        self.alt_text = text.into();
        self
    }
}

/// A spinner of the style's size in the `Ui`'s text colour.
pub fn spinner(ui: &mut Ui) -> Response {
    ui.add(Spinner::new())
}

impl Widget for Spinner {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let l = [&*t.style.spinner];
        let side = self.size.unwrap_or_else(|| own_size(&t));
        let (rect, resp) = crate::layout::allocate_item(ui, Vec2::splat(side), Sense::hover());
        let enabled = ui.is_enabled();
        report_status(&resp, enabled, &self.alt_text);
        if ui.is_rect_visible(rect) {
            let color = self.color.unwrap_or(ui.visuals().text_color());
            let angle = spin(ui, &l);
            let loader = &t.icons.loader;
            paint_icon_rotated(
                ui.painter(),
                rect,
                loader,
                color,
                t.metrics.icon_stroke,
                angle,
            );
        }
        resp
    }
}

/// Upstream's `aria-label` default.
pub(crate) const LOADING: &str = "Loading";

/// The spinner recipe's own side (`size-4`).
pub(crate) fn own_size(t: &Theme) -> f32 {
    let l = [&*t.style.spinner];
    pick(&l, |r| &r.height, State::NONE).unwrap_or(t.metrics.icon)
}

/// `role="status"`, with its implicit `aria-live="polite"`, named `name`.
fn report_status(resp: &Response, enabled: bool, name: &str) {
    widget_info_with_role(resp, Role::Status, || {
        WidgetInfo::labeled(WidgetType::ProgressIndicator, enabled, name)
    });
    resp.ctx
        .accesskit_node_builder(resp.id, |node| node.set_live(Live::Polite));
}

/// The status node of a spinner a host draws in its icon slot
/// ([`Glyph::Spinner`]): a child of the host's node, named "Loading", as
/// upstream's `<svg role="status">` stays a node inside the button.
pub(crate) fn report_hosted(ui: &mut Ui, host: &Response) {
    let child = ui.new_child(
        UiBuilder::new()
            .id_salt(host.id.with("spinner"))
            .max_rect(Rect::from_min_size(host.rect.min, Vec2::ZERO))
            .accessibility_parent(host.id),
    );
    let id = child.unique_id();
    let enabled = host.enabled();
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(Role::Status);
        node.set_label(LOADING);
        node.set_live(Live::Polite);
        if !enabled {
            node.set_disabled();
        }
    });
}

/// The angle in radians a part with `animate-spin` stands at in this
/// frame: one turn per `duration-*` along its `ease-*`, from the start of
/// the app's clock. The caller paints it visible, so it asks for the next
/// frame. Under reduced motion (`style.animation_time == 0`), or without
/// `animate-spin`, it is 0 and asks for nothing.
pub(crate) fn spin(ui: &Ui, l: &[&Recipe]) -> f32 {
    let spins = pick(l, |r| &r.animate_spin, State::NONE).unwrap_or(false);
    let period = pick(l, |r| &r.duration, State::NONE).filter(|d| *d > 0.0);
    let (true, Some(period)) = (spins, period) else {
        return 0.0;
    };
    if ui.ctx().global_style().animation_time <= 0.0 {
        return 0.0;
    }
    ui.ctx().request_repaint();
    let easing = pick(l, |r| &r.easing, State::NONE).unwrap_or(Easing::Linear); // behaviour-ok: animate-spin's own timing function
    let turn = (ui.input(|i| i.time) / f64::from(period)).fract() as f32;
    easing.at(turn) * std::f32::consts::TAU
}
