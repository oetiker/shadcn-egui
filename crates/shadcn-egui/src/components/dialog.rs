//! shadcn Dialog, ported from shadcn-svelte `ui/dialog/*.svelte` at
//! `tools/stylegen/PIN`. Width, padding, gap, radius, ring, text, the footer's
//! strip and gap, and the close button's position come from the style's
//! `cn-dialog-*` recipes.
//!
//! Built on `egui::Modal`, which keeps backdrop and panel in one area (so their
//! z-order cannot flip) and blocks input to everything beneath. Closes on Esc,
//! a backdrop click, or the close button.
//!
//! The overlay's `backdrop-blur-xs` is not drawn: egui cannot blur what lies
//! beneath, so only the overlay colour applies.

use crate::components::button::{Button, ButtonSize, ButtonVariant};
use crate::components::shared::{opacity, part_ui, RecipeBox};
use crate::style::{pick, Recipe, State, TextSize, Weight};
use crate::theme::heading_family_for;
use crate::{layout, Theme};
use egui::{Align, Color32, Id, Layout, Margin, Rect, RichText, Shape, Ui, UiBuilder};

pub struct Dialog<'a> {
    id: Id,
    title: &'a str,
    description: Option<&'a str>,
    width: Option<f32>,
    backdrop: Option<Color32>,
    show_close_button: bool,
}

impl<'a> Dialog<'a> {
    pub fn new(id_salt: impl std::hash::Hash, title: &'a str) -> Self {
        Self {
            id: Id::new(id_salt),
            title,
            description: None,
            width: None,
            backdrop: None,
            show_close_button: true,
        }
    }
    pub fn description(mut self, text: &'a str) -> Self {
        self.description = Some(text);
        self
    }
    /// Width of the dialog box, padding included; default the style's
    /// `max-w-*`, capped to keep clear of the window edges.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Backdrop color; default the style's overlay. Under a CPU rasterizer a
    /// translucent full-window backdrop is re-blended every frame the dialog
    /// is open; pass `Color32::TRANSPARENT` and dim the background once in the
    /// compositor instead (see `cpu-rendering.md`). It still blocks clicks.
    pub fn backdrop(mut self, color: Color32) -> Self {
        self.backdrop = Some(color);
        self
    }
    /// Whether the close button shows in the corner; default true.
    pub fn show_close_button(mut self, yes: bool) -> Self {
        self.show_close_button = yes;
        self
    }

    /// Show the dialog while `*open`; clears `open` when the user dismisses it.
    /// Returns the body's value while open.
    pub fn show<R>(
        self,
        ctx: &egui::Context,
        open: &mut bool,
        body: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        if !*open {
            return None;
        }
        let t = Theme::current(ctx);
        let g = t.state(ctx, State::NONE);
        let l = [&*t.style.dialog_content];
        let px = |f: fn(&Recipe) -> &crate::style::Prop<f32>| pick(&l, f, g).unwrap_or(0.0);
        let (pad_t, pad_r, pad_b, pad_l) = (
            px(|r| &r.pad_top),
            px(|r| &r.pad_right),
            px(|r| &r.pad_bottom),
            px(|r| &r.pad_left),
        );
        let gap = px(|r| &r.gap);
        let fg = pick(&l, |r| &r.text, g).map(|p| t.paint(p));
        let text_px = t.text_px(pick(&l, |r| &r.text_size, g).unwrap_or(TextSize::Sm));
        // The frame paints outside the panel's `Ui`: fade its colours here.
        let alpha = opacity(&[&l[..]], g);

        let max_width = self
            .width
            .or_else(|| pick(&l, |r| &r.max_width, g))
            .unwrap_or(f32::INFINITY);
        let width =
            layout::overlay_width(ctx, max_width, px(|r| &r.max_width_inset), pad_l + pad_r);
        let inner_width = width - pad_l - pad_r;

        let panel = RecipeBox::resolve(&t, &l, g).faded(alpha);
        let frame = panel.frame(Margin {
            top: pad_t.round() as i8,
            right: pad_r.round() as i8,
            bottom: pad_b.round() as i8,
            left: pad_l.round() as i8,
        });
        let overlay_l = [&*t.style.dialog_overlay];
        let overlay = pick(&overlay_l, |r| &r.bg, g).map_or(Color32::TRANSPARENT, |p| t.paint(p));
        // The overlay's opacity fades the backdrop, the app's colour too.
        let backdrop = self
            .backdrop
            .unwrap_or(overlay)
            .gamma_multiply(opacity(&[&overlay_l[..]], g));

        let resp = egui::Modal::new(self.id)
            .backdrop_color(backdrop)
            .frame(frame)
            .show(ctx, |ui| {
                ui.multiply_opacity(alpha);
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
                header(ui, self.title, self.description);
                let inner = body(ui);
                let close = self.show_close_button && close_button(ui, &t, (pad_t, pad_r));
                (inner, close)
            });
        // Shadow and `ring-1` lie outside the box.
        ctx.layer_painter(resp.response.layer_id)
            .extend(panel.outside_shapes(ctx, resp.response.rect));
        let dismissed = resp.backdrop_response.clicked() || resp.should_close();
        let (inner, close) = resp.inner;
        if close || dismissed {
            *open = false;
        }
        Some(inner)
    }
}

/// `dialog-header` with its title and description.
fn header(ui: &mut Ui, title: &str, description: Option<&str>) {
    let t = Theme::current(ui.ctx());
    let g = t.state(ui.ctx(), State::NONE);
    let gap = pick(&[&*t.style.dialog_header], |r| &r.gap, g).unwrap_or(0.0);
    let content = [&*t.style.dialog_content];
    let heading = [&*t.style.dialog_title];
    let size = pick(&heading, |r| &r.text_size, g)
        .or_else(|| pick(&content, |r| &r.text_size, g))
        .unwrap_or(TextSize::Sm);
    let px = t.text_px(size);
    let line = pick(&heading, |r| &r.line_height, g).map_or_else(|| t.line_px(size), |k| k * px);
    let weight = pick(&heading, |r| &r.weight, g).unwrap_or(Weight::Normal);
    let desc = [&*t.style.dialog_description];
    let desc_size = pick(&desc, |r| &r.text_size, g).unwrap_or(TextSize::Sm);
    let desc_line = pick(&desc, |r| &r.line_height, g)
        .map_or_else(|| t.line_px(desc_size), |k| k * t.text_px(desc_size));
    // The title and the description sit in the header.
    let header_l = [&*t.style.dialog_header];
    ui.scope(|ui| {
        // The header holds the title and description: they take one fade out.
        let disabled = g.with(State::DISABLED, !ui.is_enabled());
        ui.multiply_opacity(opacity(&[&header_l[..]], disabled));
        ui.spacing_mut().item_spacing.y = gap;
        // upstream: dialog-title carries `cn-font-heading`
        part_ui(ui, &[&heading[..]], g, |ui| {
            ui.label(
                RichText::new(title)
                    .family(heading_family_for(ui.ctx(), weight))
                    .size(px)
                    .line_height(Some(line)),
            )
        });
        if let Some(d) = description {
            let mut text = RichText::new(d)
                .size(t.text_px(desc_size))
                .line_height(Some(desc_line));
            if let Some(c) = pick(&desc, |r| &r.text, g).map(|p| t.paint(p)) {
                text = text.color(c);
            }
            part_ui(ui, &[&desc[..]], g, |ui| ui.label(text));
        }
    });
}

/// The close button in the panel's top-right corner, `inset_*` from its
/// edges, painted with `cn-dialog-close` over its button recipes. Returns
/// true when clicked. It is placed without moving the cursor,
/// so it does not grow the panel.
fn close_button(ui: &mut Ui, t: &Theme, pad: (f32, f32)) -> bool {
    let g = t.state(ui.ctx(), State::NONE);
    let at = [&*t.style.dialog_close];
    let (top, right) = (
        pick(&at, |r| &r.inset_top, g).unwrap_or(0.0),
        pick(&at, |r| &r.inset_right, g).unwrap_or(0.0),
    );
    let inner = ui.max_rect();
    let rect = Rect::from_min_max(
        egui::pos2(inner.left(), inner.top() - pad.0 + top),
        egui::pos2(inner.right() + pad.1 - right, inner.bottom()),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::right_to_left(Align::Min))
            .id_salt("shadcn_egui_dialog_close"),
    );
    child
        .add(
            Button::new("Close")
                .icon(&t.icons.close)
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::IconSm)
                .extra_layer(|s| &s.dialog_close),
        )
        .clicked()
}

/// `dialog-footer`: actions aligned right, primary action last (add it first:
/// the row runs right to left). In a style with a footer strip (nova) the
/// strip bleeds to the dialog's edges and bottom.
pub fn footer(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    let t = Theme::current(ui.ctx());
    let g = t.state(ui.ctx(), State::NONE);
    let l = [&*t.style.dialog_footer];
    let px = |f: fn(&Recipe) -> &crate::style::Prop<f32>| pick(&l, f, g).unwrap_or(0.0);
    let (pad_t, pad_r, pad_b, pad_l) = (
        px(|r| &r.pad_top),
        px(|r| &r.pad_right),
        px(|r| &r.pad_bottom),
        px(|r| &r.pad_left),
    );
    let (m_r, m_b, m_l) = (
        px(|r| &r.margin_right),
        px(|r| &r.margin_bottom),
        px(|r| &r.margin_left),
    );
    let gap = pick(&l, |r| &r.gap, g).unwrap_or(0.0);
    let alpha = opacity(&[&l[..]], g);

    let at = ui.max_rect();
    let top = ui.cursor().top();
    let (left, right) = (at.left() + m_l, at.right() - m_r);
    let strip_bg = ui.painter().add(Shape::Noop);
    let inner = Rect::from_min_max(
        egui::pos2(left + pad_l, top + pad_t),
        egui::pos2(right - pad_r, top + pad_t + ui.spacing().interact_size.y),
    );
    let mut row = ui.new_child(
        UiBuilder::new()
            .max_rect(inner)
            .layout(Layout::right_to_left(Align::Center)),
    );
    row.spacing_mut().item_spacing.x = gap;
    row.multiply_opacity(alpha);
    add(&mut row);
    let bottom = row.min_rect().bottom().max(inner.top()) + pad_b;

    let strip = Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
    let mut painter = ui.painter().clone();
    painter.multiply_opacity(alpha);
    let shapes = RecipeBox::resolve(&t, &l, g).shapes(ui.ctx(), strip);
    painter.set(strip_bg, Shape::Vec(shapes));
    // The negative bottom margin hands the panel's own bottom padding to the strip.
    ui.allocate_rect(
        Rect::from_min_max(
            egui::pos2(at.left(), top),
            egui::pos2(at.right(), bottom + m_b),
        ),
        egui::Sense::hover(),
    );
}
