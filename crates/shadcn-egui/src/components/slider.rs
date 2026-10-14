//! shadcn Slider, ported from shadcn-svelte `ui/slider/slider.svelte` at
//! `tools/stylegen/PIN`, which wraps the bits-ui slider. Base classes are
//! quoted under `// base:`; all other values come from the style's
//! `cn-slider` (root), `cn-slider-track`, `cn-slider-range` and
//! `cn-slider-thumb` recipes.
//!
//! The thumb stays inside the track: its centre travels from half a thumb
//! after the track start to half a thumb before its end, as in bits-ui.

use crate::components::shared::{box_shapes, disabled_painter, interaction_state, ring_shape};
use crate::style::{pick, Paint, Prop, Recipe, State};
use crate::Theme;
use egui::{pos2, vec2, EventFilter, Key, Modifiers, Rect, Sense, Shape, Stroke, Ui, Widget};

/// shadcn `orientation` of the slider.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SliderOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// A slider with one thumb (bits-ui also supports several; this port does
/// not). The value is clamped to the range on every frame and snapped to
/// `step` when the user changes it. A vertical slider has its minimum at the
/// bottom.
///
/// Keys while focused: Left/Down and Right/Up move one step (without a
/// step, 1% of the range), PageDown/PageUp ten steps, Home/End jump to the
/// ends. The returned response's rect is the slider's root box; its click
/// target also covers the thumb's hit area.
pub struct Slider<'a> {
    value: &'a mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    orientation: SliderOrientation,
    enabled: bool,
    label: String,
}

impl<'a> Slider<'a> {
    pub fn new(value: &'a mut f32, range: std::ops::RangeInclusive<f32>) -> Self {
        Self {
            value,
            range,
            step: 0.0,
            orientation: SliderOrientation::Horizontal,
            enabled: true,
            label: String::new(),
        }
    }
    /// Snap to multiples of `step` from the range start; 0 = continuous.
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }
    pub fn orientation(mut self, o: SliderOrientation) -> Self {
        self.orientation = o;
        self
    }
    /// `false` draws the slider disabled: no hover, no focus, no changes.
    pub fn enabled(mut self, yes: bool) -> Self {
        self.enabled = yes;
        self
    }
    /// Accessible name (shadcn sets it through a label).
    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.label = text.into();
        self
    }
}

/// Snap `v` to the step grid starting at `lo`, then clamp into `lo..=hi`.
fn snap(v: f32, lo: f32, hi: f32, step: f32) -> f32 {
    let v = if v.is_nan() { lo } else { v };
    let v = if step > 0.0 && step.is_finite() {
        lo + ((v - lo) / step).round() * step
    } else {
        v
    };
    v.clamp(lo, hi)
}

impl Widget for Slider<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::current(ui.ctx());
        let root = [&*t.style.slider];
        let track = [&*t.style.slider_track];
        let range = [&*t.style.slider_range];
        let thumb = [&*t.style.slider_thumb];
        let enabled = self.enabled && ui.is_enabled();
        let vertical = self.orientation == SliderOrientation::Vertical;
        let orient = if vertical {
            State::VERTICAL
        } else {
            State::HORIZONTAL
        };

        // A degenerate range is a slider that cannot move.
        let lo = if self.range.start().is_finite() {
            *self.range.start()
        } else {
            0.0
        };
        let hi = if self.range.end().is_finite() {
            self.range.end().max(lo)
        } else {
            lo
        };
        let step = self.step;

        // Geometry depends on neither the value nor interaction.
        let g = t.state(orient);
        let size_of = |l: &[&Recipe]| {
            vec2(
                pick(l, |r| &r.width, g).unwrap_or(0.0),
                pick(l, |r| &r.height, g).unwrap_or(0.0),
            )
        };
        let thumb_size = size_of(&thumb);
        let thickness = if vertical {
            size_of(&track).x
        } else {
            size_of(&track).y
        };
        let (along, cross) = if vertical {
            (thumb_size.y, thumb_size.x)
        } else {
            (thumb_size.x, thumb_size.y)
        };
        let cross = cross.max(thickness);
        let size = if vertical {
            vec2(
                cross,
                pick(&root, |r| &r.min_height, g).unwrap_or(0.0).max(along),
            )
        } else {
            // base: "relative flex w-full touch-none items-center select-none data-disabled:opacity-50 data-vertical:h-full data-vertical:w-auto data-vertical:flex-col"
            let width = ui.available_width();
            vec2(
                if width.is_finite() {
                    width.max(along)
                } else {
                    along
                },
                cross,
            )
        };
        let (rect, base) = ui.allocate_exact_size(size, Sense::hover());

        // Axis helpers: `a` runs along the slider from the minimum end.
        let (start, len) = if vertical {
            (rect.bottom(), rect.height())
        } else {
            (rect.left(), rect.width())
        };
        let dir = if vertical { -1.0 } else { 1.0 };
        let travel = (len - along).max(0.0);
        let centre_of = |frac: f32| start + dir * (along / 2.0 + frac * travel);
        let frac_of = |v: f32| {
            if hi > lo {
                ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        let thumb_at = |frac: f32| {
            let c = centre_of(frac);
            let centre = if vertical {
                pos2(rect.center().x, c)
            } else {
                pos2(c, rect.center().y)
            };
            Rect::from_center_size(centre, thumb_size)
        };

        // The value is always clamped; NaN becomes the minimum.
        let mut value = snap(*self.value, lo, hi, 0.0);
        let hit_inset = vec2(
            pick(&thumb, |r| &r.hit_inset_x, g).unwrap_or(0.0),
            pick(&thumb, |r| &r.hit_inset_y, g).unwrap_or(0.0),
        );
        let hit = rect.union(thumb_at(frac_of(value)).expand2(hit_inset));
        let sense = if enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let mut resp = ui.interact(hit, base.id.with("hit"), sense);

        let mut moved = false;
        if enabled && hi > lo {
            if resp.is_pointer_button_down_on() || resp.dragged() || resp.clicked() {
                if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
                    let a = if vertical { p.y } else { p.x };
                    let frac = if travel > 0.0 {
                        ((a - centre_of(0.0)) / (dir * travel)).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    value = snap(lo + frac * (hi - lo), lo, hi, step);
                    moved = true;
                }
            }
            if resp.has_focus() {
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        resp.id,
                        EventFilter {
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            ..Default::default()
                        },
                    )
                });
                // Without a step, bits-ui moves by 1% of the range.
                let default_steps = 100.0; // behaviour-ok: bits-ui's 1% default step, not a design token
                let unit = if step > 0.0 && step.is_finite() {
                    step
                } else {
                    (hi - lo) / default_steps
                };
                let presses = |keys: &[Key]| {
                    ui.input_mut(|i| {
                        keys.iter()
                            .map(|k| i.consume_key(Modifiers::NONE, *k) as i32)
                            .sum::<i32>()
                    })
                };
                let page_steps = 10; // behaviour-ok: bits-ui's PageUp/PageDown move ten steps, not a design token
                let steps = presses(&[Key::ArrowRight, Key::ArrowUp])
                    - presses(&[Key::ArrowLeft, Key::ArrowDown])
                    + page_steps * (presses(&[Key::PageUp]) - presses(&[Key::PageDown]));
                if steps != 0 {
                    value = snap(value + steps as f32 * unit, lo, hi, step);
                    moved = true;
                }
                if presses(&[Key::Home]) > 0 {
                    value = lo;
                    moved = true;
                }
                if presses(&[Key::End]) > 0 {
                    value = hi;
                    moved = true;
                }
            }
        }
        let changed = moved && value != *self.value;
        *self.value = value;
        if changed {
            resp.mark_changed();
        }

        let thumb_rect = thumb_at(frac_of(value));
        let pointer_on_thumb = ui
            .input(|i| i.pointer.hover_pos())
            .is_some_and(|p| thumb_rect.expand2(hit_inset).contains(p));
        let st = interaction_state(&t, &resp, enabled, orient);
        let st = st.with(
            State::HOVER,
            st.contains(State::HOVER) && (pointer_on_thumb || resp.dragged()),
        );

        if ui.is_rect_visible(hit) {
            // base: "data-disabled:opacity-50"
            let painter = disabled_painter(ui, self.enabled, 0.5);
            let paint =
                |l: &[&Recipe], f: fn(&Recipe) -> &Prop<Paint>| pick(l, f, st).map(|p| t.paint(p));

            let track_rect = if vertical {
                Rect::from_center_size(rect.center(), vec2(thickness, rect.height()))
            } else {
                Rect::from_center_size(rect.center(), vec2(rect.width(), thickness))
            };
            let track_radius =
                pick(&track, |r| &r.radius, st).map_or(0.0, |r| t.radius_px(r, thickness / 2.0));
            let mut shapes = box_shapes(
                track_rect,
                track_radius,
                paint(&track, |r| &r.bg),
                None,
                false,
            );

            let c = centre_of(frac_of(value));
            let range_rect = if vertical {
                Rect::from_min_max(pos2(track_rect.left(), c), track_rect.right_bottom())
            } else {
                Rect::from_min_max(track_rect.left_top(), pos2(c, track_rect.bottom()))
            };
            shapes.extend(box_shapes(
                range_rect,
                track_radius,
                paint(&range, |r| &r.bg),
                None,
                false,
            ));

            if let Some(s) = pick(&thumb, |r| &r.shadow, st) {
                shapes.push(Shape::from(
                    t.shadow(s)
                        .as_shape(thumb_rect, thumb_size.min_elem() / 2.0),
                ));
            }
            let border_w = pick(&thumb, |r| &r.border_width, st).unwrap_or(0.0);
            let border = paint(&thumb, |r| &r.border_color).map(|b| Stroke::new(border_w, b));
            let thumb_radius = pick(&thumb, |r| &r.radius, st)
                .map_or(0.0, |r| t.radius_px(r, thumb_size.min_elem() / 2.0));
            let clip_padding = pick(&thumb, |r| &r.bg_clip_padding, st) == Some(true);
            shapes.extend(box_shapes(
                thumb_rect,
                thumb_radius,
                paint(&thumb, |r| &r.bg),
                border,
                clip_padding,
            ));
            if let Some(c) = paint(&thumb, |r| &r.ring_color) {
                shapes.extend(ring_shape(
                    thumb_rect,
                    thumb_radius,
                    pick(&thumb, |r| &r.ring_width, st).unwrap_or(0.0),
                    c,
                ));
            }
            painter.extend(shapes);
        }
        let label = self.label;
        resp.widget_info(|| egui::WidgetInfo::slider(enabled, value as f64, label.as_str()));
        resp.rect = rect;
        resp
    }
}
