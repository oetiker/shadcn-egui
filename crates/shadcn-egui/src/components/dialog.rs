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
use crate::components::shared::{content_ui, opacity, part_painter, part_ui, RecipeBox};
use crate::components::text::{content_scope, TextLook};
use crate::style::{pick, Recipe, State};
use crate::{layout, Theme};
use egui::{Align, Color32, Id, Layout, Rect, Shape, TextWrapMode, Ui, UiBuilder};

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
        let frame = panel.frame([pad_t, pad_r, pad_b, pad_l]);
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
                let slot = RecipeBox::reserve(ui);
                ui.set_width(inner_width);
                ui.spacing_mut().item_spacing.y = gap;
                // The panel's `text-*` and text colour reach its content.
                let look = TextLook::resolve(ui, &l, g);
                content_scope(ui, &look, |ui| {
                    content_ui(ui, &[&l[..]], g, |ui| {
                        header(ui, self.title, self.description);
                        let inner = body(ui);
                        let close = self.show_close_button && close_button(ui, &t, (pad_t, pad_r));
                        (inner, close, slot)
                    })
                })
                .inner
            });
        let dismissed = resp.backdrop_response.clicked() || resp.should_close();
        let (inner, close, slot) = resp.inner;
        panel.paint_reserved(ctx, resp.response.layer_id, slot, resp.response.rect);
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
    let heading = [&*t.style.dialog_title];
    // upstream: dialog-title carries `cn-font-heading`; without a `text-*`
    // it takes the panel's, which the content scope passes.
    let title_look = TextLook::resolve(ui, &heading, g).heading(true);
    let desc = [&*t.style.dialog_description];
    let desc_look = TextLook::resolve(ui, &desc, g);
    // The title and the description sit in the header.
    let header_l = [&*t.style.dialog_header];
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = gap;
        content_ui(ui, &[&header_l[..]], g, |ui| {
            part_ui(ui, &[&heading[..]], g, |ui| {
                title_look.label(ui, title, TextWrapMode::Wrap)
            });
            if let Some(d) = description {
                part_ui(ui, &[&desc[..]], g, |ui| {
                    desc_look.label(ui, d, TextWrapMode::Wrap)
                });
            }
        });
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
    content_ui(&mut row, &[&l[..]], g, add);
    let bottom = row.min_rect().bottom().max(inner.top()) + pad_b;

    let strip = Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom));
    let painter = part_painter(ui, &[&l[..]], g);
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
