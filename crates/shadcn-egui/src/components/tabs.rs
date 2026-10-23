//! shadcn Tabs, ported from shadcn-svelte `ui/tabs/*.svelte` at
//! `tools/stylegen/PIN`. Every value comes from the style's `cn-tabs-list`,
//! `cn-tabs-list-variant-*`, `cn-tabs-trigger` recipes and the trigger's
//! `after:` recipe (the line indicator). Behaviour follows bits-ui Tabs: a press focuses and selects,
//! Left/Right move the selection (and wrap), Home/End jump to the first/last
//! tab (automatic activation). The list is one Tab stop, on the selected
//! trigger (roving focus).

use crate::components::shared::{
    interaction_state, line_box, opacity, part_painter, pressable, ActivationKeys, RecipeBox,
};
use crate::style::{pick, Paint, Prop, Recipe, Role, State, TextSize, Weight};
use crate::theme::family_for;
use crate::{layout, Theme};
use egui::{pos2, vec2, Color32, Key, Modifiers, Rect, Response, Sense, Shape, Ui, Widget};

/// shadcn `TabsList` `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabsVariant {
    /// A muted track with the active trigger as a raised pill.
    #[default]
    Default,
    /// No track; the active trigger gets an underline.
    Line,
}

/// shadcn `Tabs` `orientation`. The crate offers `Horizontal` only: a vertical
/// list needs a content layout beside the list that the crate does not have.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum TabsOrientation {
    #[default]
    Horizontal,
}

/// A tab list (shadcn `TabsList` with its `TabsTrigger`s). By default the list
/// is as wide as its triggers and each trigger as wide as its label;
/// [`Tabs::width`] makes it wider. The response is the
/// list's, marked changed when the selection moved. Content below the list
/// sits [`Tabs::content_gap`] under it (shadcn `Tabs` root `gap`).
pub struct Tabs<'a> {
    active: &'a mut usize,
    labels: Vec<String>,
    variant: TabsVariant,
    orientation: TabsOrientation,
    enabled: bool,
    width: Option<f32>,
}

impl<'a> Tabs<'a> {
    /// `active` is the selected index into `labels`.
    pub fn new(active: &'a mut usize, labels: &[&str]) -> Self {
        Self {
            active,
            labels: labels.iter().map(|l| (*l).to_owned()).collect(),
            variant: TabsVariant::Default,
            orientation: TabsOrientation::Horizontal,
            enabled: true,
            width: None,
        }
    }
    pub fn variant(mut self, v: TabsVariant) -> Self {
        self.variant = v;
        self
    }
    pub fn orientation(mut self, o: TabsOrientation) -> Self {
        self.orientation = o;
        self
    }
    /// `false` draws every trigger disabled: no hover, no focus, no selecting.
    pub fn enabled(mut self, yes: bool) -> Self {
        self.enabled = yes;
        self
    }
    /// List width in points, padding included (shadcn `class="w-full"`
    /// on `TabsList`). Pass `f32::INFINITY` for full-width. The width is
    /// capped to the available width and never smaller than the triggers'
    /// own; the triggers share it as their `flex-1` does: equal parts, and a
    /// trigger whose label needs more keeps its own width. A NaN, zero or
    /// negative width is ignored, and so is `f32::INFINITY` in a `Ui` of
    /// unbounded width: the list then stays as wide as its triggers. In a
    /// `ScrollArea`, `f32::INFINITY` fills the visible width.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// The gap between the list and the content below it (shadcn `Tabs` root `gap`).
    pub fn content_gap(ui: &Ui) -> f32 {
        let t = Theme::current(ui.ctx());
        pick(
            &[&*t.style.tabs],
            |r| &r.gap,
            t.state(ui.ctx(), State::NONE),
        )
        .unwrap_or(0.0)
    }
}

impl Widget for Tabs<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let s = &t.style;
        let enabled = self.enabled && ui.is_enabled();
        let (variant_bit, variant_layer) = match self.variant {
            TabsVariant::Default => (State::VARIANT_DEFAULT, &*s.tabs_list_variant_default),
            TabsVariant::Line => (State::VARIANT_LINE, &*s.tabs_list_variant_line),
        };
        let TabsOrientation::Horizontal = self.orientation;
        let root = [&*s.tabs];
        let list = [&*s.tabs_list, variant_layer];
        let trigger = [&*s.tabs_trigger];
        let after = [&*s.tabs_trigger_after];

        // Geometry depends neither on the selection nor on interaction.
        let g = t.state(ui.ctx(), State::HORIZONTAL.or(variant_bit));
        let geo = |l: &[&Recipe], f: fn(&Recipe) -> &Prop<f32>| pick(l, f, g).unwrap_or(0.0);
        let (pad_t, pad_r, pad_b, pad_l) = (
            geo(&list, |r| &r.pad_top),
            geo(&list, |r| &r.pad_right),
            geo(&list, |r| &r.pad_bottom),
            geo(&list, |r| &r.pad_left),
        );
        let (tpad_l, tpad_r, border_w) = (
            geo(&trigger, |r| &r.pad_left),
            geo(&trigger, |r| &r.pad_right),
            geo(&trigger, |r| &r.border_width),
        );
        let gap = geo(&list, |r| &r.gap);
        let text_size = pick(&trigger, |r| &r.text_size, g).unwrap_or(TextSize::Sm);
        let text_px = t.text_px(text_size);
        let weight = pick(&trigger, |r| &r.weight, g).unwrap_or(Weight::Normal);
        let font = egui::FontId::new(text_px, family_for(ui.ctx(), weight));
        let painter = ui.painter().clone();
        // The text colour depends on the state: lay out in egui's placeholder
        // colour, paint in the final one.
        let galleys: Vec<_> = self
            .labels
            .iter()
            .map(|l| painter.layout_no_wrap(l.clone(), font.clone(), Color32::PLACEHOLDER))
            .collect();
        // No list height: CSS's initial `auto`, the triggers' own height,
        // which `h-[calc(100%-1px)]` of an `auto` parent leaves at `auto` too.
        let trigger_auto = galleys
            .iter()
            .map(|g| g.size().y)
            .fold(line_box(&t, &trigger, g, text_size), f32::max)
            + geo(&trigger, |r| &r.pad_top)
            + geo(&trigger, |r| &r.pad_bottom)
            + border_w * 2.0;
        let (list_h, trigger_h) = match pick(&list, |r| &r.height, g) {
            Some(h) => (h, h - pad_t - pad_b - geo(&trigger, |r| &r.height_inset)),
            None => (trigger_auto + pad_t + pad_b, trigger_auto),
        };
        let widths: Vec<f32> = galleys
            .iter()
            .map(|g| g.size().x + tpad_l + tpad_r + border_w * 2.0)
            .collect();
        let n = widths.len();
        let chrome = gap * n.saturating_sub(1) as f32 + pad_l + pad_r;
        let hug_w = widths.iter().sum::<f32>() + chrome;
        // A NaN width, one below the triggers' own, or an unbounded `Ui`
        // leaves the list as wide as its triggers.
        let list_w = self
            .width
            .map_or(hug_w, |w| layout::capped_width(ui, w, hug_w));
        let widths = if list_w > hug_w {
            flex_shares(&widths, list_w - chrome)
        } else {
            widths
        };

        // Wider than its room, the list overflows as `w-fit` does in the
        // browser: the column keeps its width.
        let (list_rect, mut resp) =
            layout::allocate_overflowing(ui, vec2(list_w, list_h), Sense::hover());
        let ids: Vec<egui::Id> = (0..n).map(|i| resp.id.with(("tab", i))).collect();
        let mut x = list_rect.left() + pad_l;
        let trigger_rects: Vec<Rect> = widths
            .iter()
            .map(|w| {
                let r = Rect::from_min_size(
                    pos2(x, list_rect.center().y - trigger_h / 2.0),
                    vec2(*w, trigger_h),
                );
                x += w + gap;
                r
            })
            .collect();
        // Roving focus: the list is one Tab stop, on the selected trigger.
        // A trigger that holds the focus keeps it, so the focus survives
        // the app moving the selection.
        let resps: Vec<Response> = trigger_rects
            .iter()
            .zip(&ids)
            .enumerate()
            .map(|(i, (r, id))| {
                let sense = if !enabled {
                    Sense::hover()
                } else if i == *self.active || ui.memory(|m| m.has_focus(*id)) {
                    Sense::click()
                } else {
                    Sense::CLICK
                };
                let mut resp = ui.interact(*r, *id, sense);
                pressable(&mut resp, ActivationKeys::EnterAndSpace);
                resp
            })
            .collect();

        // Selection: a click, focus (bits' automatic activation, so a press
        // selects as it focuses), or the arrow/Home/End keys on a focused
        // trigger, which move the focus along.
        let mut next = None;
        let mut by_key = false;
        if enabled {
            for (i, r) in resps.iter().enumerate() {
                if r.clicked() || r.gained_focus() {
                    next = Some(i);
                }
                if r.has_focus() {
                    // Keep egui from using the arrows to move focus on its own.
                    ui.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            r.id,
                            egui::EventFilter {
                                horizontal_arrows: true,
                                ..Default::default()
                            },
                        )
                    });
                    let pressed = |key| ui.input_mut(|inp| inp.consume_key(Modifiers::NONE, key));
                    let key_next = if pressed(Key::ArrowRight) {
                        Some((i + 1) % n)
                    } else if pressed(Key::ArrowLeft) {
                        Some((i + n - 1) % n)
                    } else if pressed(Key::Home) {
                        Some(0)
                    } else if pressed(Key::End) {
                        Some(n - 1)
                    } else {
                        None
                    };
                    if key_next.is_some() {
                        next = key_next;
                        by_key = true;
                    }
                }
            }
        }
        if let Some(i) = next {
            if *self.active != i {
                *self.active = i;
                resp.mark_changed();
            }
            if by_key {
                ui.memory_mut(|m| m.request_focus(ids[i]));
            }
        }
        let active = *self.active;

        // The line indicator may reach below the list (`after:bottom-[-5px]`).
        let reach = (-geo(&after, |r| &r.inset_bottom)).max(0.0);
        for (i, r) in resps.iter().enumerate() {
            r.widget_info(|| {
                egui::WidgetInfo::selected(
                    egui::WidgetType::SelectableLabel,
                    enabled,
                    i == active,
                    &self.labels[i],
                )
            });
        }
        if ui.is_rect_visible(list_rect.expand(reach)) {
            // The list sits in the tabs root; a trigger in the list.
            RecipeBox::resolve(&t, &list, g)
                .paint(&part_painter(ui, &[&list[..], &root[..]], g), list_rect);
            let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground where neither trigger nor list sets a colour
            let list_text = pick(&list, |r| &r.text, g).map(|p| t.paint(p));

            for (i, (rect, r)) in trigger_rects.iter().zip(&resps).enumerate() {
                let context = State::HORIZONTAL
                    .or(variant_bit)
                    .with(State::SELECTED, i == active);
                let st = interaction_state(&t, r, context.with(State::DISABLED, !enabled));
                let painter = part_painter(ui, &[&trigger[..], &list[..], &root[..]], st);
                let paint = |l: &[&Recipe], f: fn(&Recipe) -> &Prop<Paint>| {
                    pick(l, f, st).map(|p| t.paint(p))
                };
                let px =
                    |l: &[&Recipe], f: fn(&Recipe) -> &Prop<f32>| pick(l, f, st).unwrap_or(0.0);

                let fg = paint(&trigger, |r| &r.text)
                    .or(list_text)
                    .unwrap_or(inherited);
                let mut shapes = RecipeBox::resolve(&t, &trigger, st).shapes(ui.ctx(), *rect);
                // The `after:` line indicator, placed against the padding box.
                // The painter already carries the trigger's chain.
                let bar_alpha = opacity(&[&after[..]], st);
                if let (Some(c), true) = (paint(&after, |r| &r.bg), bar_alpha > 0.0) {
                    let inner = rect.shrink(border_w);
                    let bottom = inner.bottom() - px(&after, |r| &r.inset_bottom);
                    let bar = Rect::from_min_max(
                        pos2(
                            inner.left() + px(&after, |r| &r.inset_left),
                            bottom - px(&after, |r| &r.height),
                        ),
                        pos2(inner.right() - px(&after, |r| &r.inset_right), bottom),
                    );
                    shapes.push(Shape::rect_filled(bar, 0.0, c.gamma_multiply(bar_alpha)));
                }
                painter.extend(shapes);
                let galley = galleys[i].clone();
                painter.galley(rect.center() - galley.size() / 2.0, galley, fg);
            }
        }
        resp
    }
}

/// The trigger widths of a list `inner` wide (padding and gaps taken off),
/// as CSS lays out `flex-1` items: equal parts of `inner`, and an item whose
/// content is wider than its part keeps its content width while the others
/// share the rest.
fn flex_shares(content: &[f32], inner: f32) -> Vec<f32> {
    let mut frozen = vec![false; content.len()];
    loop {
        let open = frozen.iter().filter(|f| !**f).count();
        if open == 0 {
            return content.to_vec();
        }
        let taken: f32 = content
            .iter()
            .zip(&frozen)
            .filter(|(_, f)| **f)
            .map(|(c, _)| c)
            .sum();
        let share = (inner - taken) / open as f32;
        let mut froze = false;
        for (c, f) in content.iter().zip(frozen.iter_mut()) {
            if !*f && *c > share {
                *f = true;
                froze = true;
            }
        }
        if !froze {
            return content
                .iter()
                .zip(&frozen)
                .map(|(c, f)| if *f { *c } else { share })
                .collect();
        }
    }
}

/// Renders a tab bar in the default variant. `active` is the selected index.
/// Returns true if it changed.
pub fn tab_bar(ui: &mut Ui, active: &mut usize, labels: &[&str]) -> bool {
    ui.add(Tabs::new(active, labels)).changed()
}
