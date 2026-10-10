//! shadcn Popover, ported from shadcn-svelte `ui/popover/*.svelte` at
//! `tools/stylegen/PIN`. Base classes are quoted under `// base:`; padding,
//! gap, radius, ring, shadow and text come from the style's
//! `cn-popover-content`, `cn-popover-header`, `cn-popover-title` and
//! `cn-popover-description` recipes.
//!
//! Behaviour follows bits-ui Popover: a click on the trigger toggles it, Esc
//! and a click outside close it. bits-ui also moves keyboard focus into the
//! content; egui does not, so focus stays where it was.
//!
//! egui remembers one open popup at a time, so opening a popover closes any
//! other popup (a menu, a combo box). Nest them only through `open_bool`
//! state of your own.

use crate::components::shared::ring;
use crate::style::{pick, State, TextSize, Weight};
use crate::theme::family_for;
use crate::Theme;
use egui::{Frame, Margin, PopupCloseBehavior, Response, RichText, Ui};

// base: "w-72"
const DEFAULT_WIDTH: f32 = 288.0;
// base: "sideOffset = 4"
const SIDE_OFFSET: f32 = 4.0;

pub struct Popover<'a> {
    trigger: &'a Response,
    width: Option<f32>,
}

impl<'a> Popover<'a> {
    /// A popover toggled by clicks on `trigger`.
    pub fn new(trigger: &'a Response) -> Self {
        Self { trigger, width: None }
    }
    /// Width of the popover box, padding included; default `w-72`.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Show the content while open. Returns its value while open.
    pub fn show<R>(self, content: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let t = Theme::current(&self.trigger.ctx);
        let g = t.state(State::NONE);
        let l = [&t.style.popover_content];
        let px = |f: fn(&crate::style::Recipe) -> &crate::style::Prop<f32>| pick(&l, f, g).unwrap_or(0.0);
        let (pad_t, pad_r, pad_b, pad_l) = (px(|r| &r.pad_top), px(|r| &r.pad_right), px(|r| &r.pad_bottom), px(|r| &r.pad_left));
        let gap = px(|r| &r.gap);
        let radius = pick(&l, |r| &r.radius, g).map_or(0.0, |r| t.radius_px(r, f32::INFINITY));
        let fill = pick(&l, |r| &r.bg, g).map(|p| t.paint(p));
        let fg = pick(&l, |r| &r.text, g).map(|p| t.paint(p));
        let ring_width = px(|r| &r.ring_width);
        let ring_color = pick(&l, |r| &r.ring_color, g).map(|p| t.paint(p));
        let shadow = pick(&l, |r| &r.shadow, g).map_or(egui::epaint::Shadow::NONE, |s| t.shadow(s));
        let text_px = t.text_px(pick(&l, |r| &r.text_size, g).unwrap_or(TextSize::Sm));

        let mut frame = Frame::new().corner_radius(radius.round() as u8).shadow(shadow).inner_margin(Margin {
            top: pad_t.round() as i8,
            right: pad_r.round() as i8,
            bottom: pad_b.round() as i8,
            left: pad_l.round() as i8,
        });
        if let Some(fill) = fill {
            frame = frame.fill(fill);
        }
        let inner_width = self.width.unwrap_or(DEFAULT_WIDTH) - pad_l - pad_r;
        let shown = egui::Popup::from_toggle_button_response(self.trigger)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .gap(SIDE_OFFSET)
            .frame(frame)
            .show(|ui| {
                ui.set_width(inner_width);
                ui.spacing_mut().item_spacing.y = gap;
                if let Some(fg) = fg {
                    ui.visuals_mut().override_text_color = Some(fg);
                }
                for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
                    if let Some(font) = ui.style_mut().text_styles.get_mut(&style) {
                        font.size = text_px;
                    }
                }
                content(ui)
            })?;
        // `ring-1` lies outside the box; the frame's stroke would lie inside.
        if let Some(color) = ring_color {
            let painter = self.trigger.ctx.layer_painter(shown.response.layer_id);
            ring(&painter, shown.response.rect, radius, ring_width, color);
        }
        Some(shown.inner)
    }
}

/// shadcn `PopoverHeader` with its `PopoverTitle` and `PopoverDescription`.
pub fn popover_header(ui: &mut Ui, title: &str, description: &str) {
    let t = Theme::current(ui.ctx());
    let g = t.state(State::NONE);
    let header = [&t.style.popover_header];
    let content = [&t.style.popover_content];
    let gap = pick(&header, |r| &r.gap, g).unwrap_or(0.0);
    let text_px = t.text_px(pick(&header, |r| &r.text_size, g).unwrap_or(TextSize::Sm));
    let weight = pick(&[&t.style.popover_title], |r| &r.weight, g).unwrap_or(Weight::Normal);
    let title_color = pick(&content, |r| &r.text, g).map(|p| t.paint(p));
    let muted = pick(&[&t.style.popover_description], |r| &r.text, g).map(|p| t.paint(p));
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = gap;
        let mut title = RichText::new(title).size(text_px).family(family_for(ui.ctx(), weight));
        if let Some(c) = title_color {
            title = title.color(c);
        }
        ui.label(title);
        let mut description = RichText::new(description).size(text_px);
        if let Some(c) = muted {
            description = description.color(c);
        }
        ui.label(description);
    });
}
