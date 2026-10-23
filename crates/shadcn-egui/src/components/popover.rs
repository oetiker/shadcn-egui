//! shadcn Popover, ported from shadcn-svelte `ui/popover/*.svelte` at
//! `tools/stylegen/PIN`. Width, padding, gap, radius, ring, shadow and text
//! come from the style's `cn-popover-content`, `cn-popover-header`,
//! `cn-popover-title` and `cn-popover-description` recipes.
//!
//! Behaviour follows bits-ui Popover: a click on the trigger toggles it, Esc
//! and a click outside close it. bits-ui also moves keyboard focus into the
//! content; egui does not, so focus stays where it was.
//!
//! egui remembers one open popup at a time, so opening a popover closes any
//! other popup (a menu, a combo box). Nest them only through `open_bool`
//! state of your own.

use crate::components::shared::{opacity, part_ui, RecipeBox, DEFAULT_SIDE_OFFSET};
use crate::style::{pick, Recipe, State, TextSize, Weight};
use crate::theme::family_for;
use crate::Theme;
use egui::{PopupCloseBehavior, Response, RichText, Ui};

pub struct Popover<'a> {
    trigger: &'a Response,
    width: Option<f32>,
    side_offset: f32,
}

impl<'a> Popover<'a> {
    /// A popover toggled by clicks on `trigger`.
    pub fn new(trigger: &'a Response) -> Self {
        Self {
            trigger,
            width: None,
            side_offset: DEFAULT_SIDE_OFFSET,
        }
    }
    /// Width of the popover box, padding included; default the style's
    /// `w-72`.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Gap between the trigger and the popover (shadcn `sideOffset`, default 4).
    pub fn side_offset(mut self, px: f32) -> Self {
        self.side_offset = px;
        self
    }
    /// Show the content while open. Returns its value while open.
    pub fn show<R>(self, content: impl FnOnce(&mut Ui) -> R) -> Option<R> {
        let t = Theme::current(&self.trigger.ctx);
        let g = t.state(&self.trigger.ctx, State::NONE);
        let l = [&*t.style.popover_content];
        let px = |f: fn(&crate::style::Recipe) -> &crate::style::Prop<f32>| {
            pick(&l, f, g).unwrap_or(0.0)
        };
        let (pad_t, pad_r, pad_b, pad_l) = (
            px(|r| &r.pad_top),
            px(|r| &r.pad_right),
            px(|r| &r.pad_bottom),
            px(|r| &r.pad_left),
        );
        let gap = px(|r| &r.gap);
        // The frame paints outside the content's `Ui`: fade its colours here.
        let alpha = opacity(&[&l[..]], g);
        let fg = pick(&l, |r| &r.text, g).map(|p| t.paint(p));
        let text_px = t.text_px(pick(&l, |r| &r.text_size, g).unwrap_or(TextSize::Sm));
        let panel = RecipeBox::resolve(&t, &l, g).faded(alpha);
        let frame = panel.frame([pad_t, pad_r, pad_b, pad_l]);
        // No width from the builder or the recipe: CSS's initial `auto`,
        // the content's width.
        let inner_width = self
            .width
            .or_else(|| pick(&l, |r| &r.width, g))
            .map(|w| (w - pad_l - pad_r).max(0.0));
        let shown = egui::Popup::from_toggle_button_response(self.trigger)
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .gap(self.side_offset)
            .frame(frame)
            .show(|ui| {
                let slot = RecipeBox::reserve(ui);
                ui.multiply_opacity(alpha);
                if let Some(w) = inner_width {
                    ui.set_width(w);
                }
                ui.spacing_mut().item_spacing.y = gap;
                if let Some(fg) = fg {
                    ui.visuals_mut().override_text_color = Some(fg);
                }
                for style in [egui::TextStyle::Body, egui::TextStyle::Button] {
                    if let Some(font) = ui.style_mut().text_styles.get_mut(&style) {
                        font.size = text_px;
                    }
                }
                (content(ui), slot)
            })?;
        let (inner, slot) = shown.inner;
        panel.paint_reserved(
            &self.trigger.ctx,
            shown.response.layer_id,
            slot,
            shown.response.rect,
        );
        Some(inner)
    }
}

/// shadcn `PopoverHeader` with its `PopoverTitle` and `PopoverDescription`.
/// Each line takes its size and line height from its own recipe, over the
/// header's.
pub fn popover_header(ui: &mut Ui, title: &str, description: &str) {
    let t = Theme::current(ui.ctx());
    let g = t.state(ui.ctx(), State::NONE);
    let header = &*t.style.popover_header;
    let title_l = [header, &*t.style.popover_title];
    let desc_l = [header, &*t.style.popover_description];
    let gap = pick(&[header], |r| &r.gap, g).unwrap_or(0.0);
    let text = |l: &[&Recipe]| {
        let size = pick(l, |r| &r.text_size, g).unwrap_or(TextSize::Sm);
        let px = t.text_px(size);
        // Tailwind's `text-*` sets a line height too; `leading-*` overrides it.
        let line = pick(l, |r| &r.line_height, g).map_or_else(|| t.line_px(size), |k| k * px);
        (px, line)
    };
    let (title_px, title_line) = text(&title_l);
    let (desc_px, desc_line) = text(&desc_l);
    let weight = pick(&title_l, |r| &r.weight, g).unwrap_or(Weight::Normal);
    let title_color = pick(&title_l, |r| &r.text, g)
        .or(pick(&[&*t.style.popover_content], |r| &r.text, g))
        .map(|p| t.paint(p));
    let muted = pick(&desc_l, |r| &r.text, g).map(|p| t.paint(p));
    // The title and the description sit in the header.
    let (title_l, desc_l2, header_l) = (
        [&*t.style.popover_title],
        [&*t.style.popover_description],
        [header],
    );
    ui.scope(|ui| {
        // The header holds the title and description: they take one fade out.
        let disabled = g.with(State::DISABLED, !ui.is_enabled());
        ui.multiply_opacity(opacity(&[&header_l[..]], disabled));
        ui.spacing_mut().item_spacing.y = gap;
        let mut title = RichText::new(title)
            .size(title_px)
            .line_height(Some(title_line))
            .family(family_for(ui.ctx(), weight));
        if let Some(c) = title_color {
            title = title.color(c);
        }
        part_ui(ui, &[&title_l[..]], g, |ui| ui.label(title));
        let mut description = RichText::new(description)
            .size(desc_px)
            .line_height(Some(desc_line));
        if let Some(c) = muted {
            description = description.color(c);
        }
        part_ui(ui, &[&desc_l2[..]], g, |ui| ui.label(description));
    });
}

#[cfg(test)]
mod tests {
    use crate::style::{pick, State, Style, TextSize};

    /// The title and description sizes come from their own recipes, which
    /// differ from the header's in lyra and maia.
    #[test]
    fn header_lines_read_their_own_recipes() {
        let lyra = Style::lyra();
        assert_eq!(
            pick(&[&lyra.popover_header], |r| &r.text_size, State::NONE),
            Some(TextSize::Xs)
        );
        assert_eq!(
            pick(
                &[&lyra.popover_header, &lyra.popover_title],
                |r| &r.text_size,
                State::NONE
            ),
            Some(TextSize::Sm)
        );
        assert_eq!(
            pick(
                &[&lyra.popover_header, &lyra.popover_description],
                |r| &r.line_height,
                State::NONE
            ),
            Some(1.625)
        );
        let maia = Style::maia();
        assert_eq!(
            pick(
                &[&maia.popover_header, &maia.popover_title],
                |r| &r.text_size,
                State::NONE
            ),
            Some(TextSize::Base)
        );
    }
}
