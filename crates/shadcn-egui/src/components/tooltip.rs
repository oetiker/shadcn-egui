//! shadcn Tooltip, ported from shadcn-svelte `ui/tooltip/tooltip-content.svelte`
//! at `tools/stylegen/PIN`. Base classes are quoted under `// base:`; the
//! padding, text size and radius come from the style's `cn-tooltip-content`
//! recipe, the arrow's size and radius from `cn-tooltip-arrow`. The arrow
//! classes `rotate-45` and `translate-y-[calc(-50%-2px)]` belong to the style
//! and stylegen ignores them: the arrow is painted here as a diamond whose
//! centre lies on the content edge, below or above the content, centred on the
//! trigger.

use crate::style::{pick, Role, State, TextSize};
use crate::Theme;
use egui::{pos2, Color32, Frame, Margin, Pos2, RectAlign, Response, RichText, Shape};
use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2};

/// Corner arc segments of the rounded arrow.
const ARC_STEPS: usize = 4;

/// Attach a shadcn tooltip to `resp`: `tooltip(ui.add(..), "Mute")`.
pub fn tooltip(resp: Response, text: &str) -> Response {
    let t = Theme::current(&resp.ctx);
    let g = t.state(State::NONE);
    let content = [&*t.style.tooltip_content];
    let arrow = [&*t.style.tooltip_arrow];
    let px = |f: fn(&crate::style::Recipe) -> &crate::style::Prop<f32>| {
        pick(&content, f, g).unwrap_or(0.0)
    };
    let (pad_t, pad_r, pad_b, pad_l) = (
        px(|r| &r.pad_top),
        px(|r| &r.pad_right),
        px(|r| &r.pad_bottom),
        px(|r| &r.pad_left),
    );
    let text_size = pick(&content, |r| &r.text_size, g).unwrap_or(TextSize::Xs);
    let text_px = t.text_px(text_size);
    // Tailwind's `text-*` sets a line height too; `leading-*` overrides it.
    let line =
        pick(&content, |r| &r.line_height, g).map_or_else(|| t.line_px(text_size), |k| k * text_px);
    let radius = pick(&content, |r| &r.radius, g).map_or(0.0, |r| t.radius_px(r, f32::INFINITY));
    // base: "bg-foreground text-background"
    let (fill, fg) = (t.role(Role::Foreground), t.role(Role::Background));
    let mut tip = egui::Tooltip::for_enabled(&resp);
    // base: "z-50 w-fit max-w-xs"
    let max_w = 320.0;
    // The arrow's tip, not the content, touches the trigger; `side = "top"` is the base default.
    let size = pick(&[&*t.style.tooltip_arrow], |r| &r.width, g).unwrap_or(0.0);
    let reach = size * FRAC_1_SQRT_2;
    tip.popup = tip.popup.align(RectAlign::TOP).gap(reach).frame(
        Frame::new()
            .fill(fill)
            .corner_radius(radius.round() as u8)
            .inner_margin(Margin {
                top: pad_t.round() as i8,
                right: pad_r.round() as i8,
                bottom: pad_b.round() as i8,
                left: pad_l.round() as i8,
            }),
    );
    let shown = tip.show(|ui| {
        ui.set_max_width(max_w - pad_l - pad_r);
        ui.label(
            RichText::new(text)
                .size(text_px)
                .line_height(Some(line))
                .color(fg),
        )
    });
    if let Some(shown) = shown {
        let r = pick(&arrow, |r| &r.radius, g).map_or(0.0, |r| t.radius_px(r, size / 2.0));
        let area = shown.response.rect;
        // The content lies above or below the trigger; the arrow sits on the edge facing it.
        let above = area.center().y < resp.rect.center().y;
        let y = if above { area.max.y } else { area.min.y };
        // Centred on the trigger, kept clear of the content's rounded corners.
        let (lo, hi) = (area.min.x + radius + reach, area.max.x - radius - reach);
        let x = if lo < hi {
            resp.rect.center().x.clamp(lo, hi)
        } else {
            area.center().x
        };
        let painter = resp.ctx.layer_painter(shown.response.layer_id);
        // base: "bg-foreground fill-foreground"
        painter.add(arrow_shape(pos2(x, y), size, r, fill));
    }
    resp
}

/// A square of side `size` with corner radius `radius`, turned 45 degrees about `centre`.
fn arrow_shape(centre: Pos2, size: f32, radius: f32, fill: Color32) -> Shape {
    let inner = size / 2.0 - radius;
    let mut points = Vec::with_capacity(4 * (ARC_STEPS + 1));
    // The corner arcs' centres, one per quadrant in turning order.
    for (k, (sx, sy)) in [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)]
        .into_iter()
        .enumerate()
    {
        for i in 0..=ARC_STEPS {
            let a = (k as f32 + i as f32 / ARC_STEPS as f32) * FRAC_PI_2;
            let (x, y) = (sx * inner + radius * a.cos(), sy * inner + radius * a.sin());
            points.push(centre + egui::vec2((x - y) * FRAC_1_SQRT_2, (x + y) * FRAC_1_SQRT_2));
        }
    }
    Shape::convex_polygon(points, fill, egui::Stroke::NONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Rect;

    #[test]
    fn arrow_is_a_diamond_with_rounded_tips() {
        let centre = pos2(50.0, 50.0);
        let Shape::Path(p) = arrow_shape(centre, 10.0, 2.0, Color32::PLACEHOLDER) else {
            panic!("path")
        };
        let bounds = Rect::from_points(&p.points);
        // A sharp 10px square turned 45 degrees reaches 7.07px; the rounded tips stop short of that.
        let reach = 10.0 * FRAC_1_SQRT_2;
        assert!(bounds.center().distance(centre) < 0.01);
        assert!(
            bounds.width() / 2.0 < reach && bounds.width() / 2.0 > reach - 1.0,
            "{bounds:?}"
        );
        assert!((bounds.width() - bounds.height()).abs() < 0.01);
    }
}
