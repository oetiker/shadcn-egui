//! shadcn Tabs, ported from shadcn-svelte `ui/tabs/*.svelte` at
//! `tools/stylegen/PIN`. Base classes are quoted under `// base:`; all other
//! values come from the style's `cn-tabs-list`, `cn-tabs-list-variant-*` and
//! `cn-tabs-trigger` recipes. Behaviour follows bits-ui Tabs: a click selects,
//! Left/Right move the selection (and wrap), Home/End jump to the first/last
//! tab (automatic activation).

use crate::components::shared::{box_shapes, disabled_painter, interaction_state, ring_shape};
use crate::style::{pick, Paint, Prop, Recipe, Role, State, TextSize, Weight};
use crate::theme::family_for;
use crate::Theme;
use egui::{pos2, vec2, Color32, Key, Modifiers, Rect, Response, Sense, Shape, Stroke, Ui, Widget};

/// shadcn `TabsList` `variant`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabsVariant {
    /// A muted track with the active trigger as a raised pill.
    #[default]
    Default,
    /// No track; the active trigger gets an underline.
    Line,
}

/// shadcn `Tabs` `orientation`. 0.4.0 offers `Horizontal` only: a vertical
/// list needs a content layout beside the list that the crate does not have.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum TabsOrientation {
    #[default]
    Horizontal,
}

/// A tab list (shadcn `TabsList` with its `TabsTrigger`s). The list is as wide
/// as its triggers; each trigger is as wide as its label. The response is the
/// list's, marked changed when the selection moved. Content below the list
/// sits [`Tabs::content_gap`] under it (shadcn `Tabs` root `gap`).
pub struct Tabs<'a> {
    active: &'a mut usize,
    labels: Vec<String>,
    variant: TabsVariant,
    orientation: TabsOrientation,
    enabled: bool,
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
    /// The gap between the list and the content below it (shadcn `Tabs` root `gap`).
    pub fn content_gap(ui: &Ui) -> f32 {
        let t = Theme::current(ui.ctx());
        pick(&[&t.style.tabs], |r| &r.gap, t.state(State::NONE)).unwrap_or(0.0)
    }
}

impl Widget for Tabs<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let t = Theme::current(ui.ctx());
        let s = &t.style;
        let enabled = self.enabled && ui.is_enabled();
        let (variant_bit, variant_layer) = match self.variant {
            TabsVariant::Default => (State::VARIANT_DEFAULT, &s.tabs_list_variant_default),
            TabsVariant::Line => (State::VARIANT_LINE, &s.tabs_list_variant_line),
        };
        let TabsOrientation::Horizontal = self.orientation;
        let list = [&s.tabs_list, variant_layer];
        let trigger = [&s.tabs_trigger];

        // Geometry depends neither on the selection nor on interaction.
        let g = t.state(State::HORIZONTAL.or(variant_bit));
        let geo = |l: &[&Recipe], f: fn(&Recipe) -> &Prop<f32>| pick(l, f, g).unwrap_or(0.0);
        let (pad_t, pad_r, pad_b, pad_l) = (
            geo(&list, |r| &r.pad_top),
            geo(&list, |r| &r.pad_right),
            geo(&list, |r| &r.pad_bottom),
            geo(&list, |r| &r.pad_left),
        );
        let list_h = geo(&list, |r| &r.height);
        // base: "h-[calc(100%-1px)]"
        let trigger_h = list_h - pad_t - pad_b - 1.0;
        let (tpad_l, tpad_r, border_w) = (
            geo(&trigger, |r| &r.pad_left),
            geo(&trigger, |r| &r.pad_right),
            geo(&trigger, |r| &r.border_width),
        );
        // base: "gap-1 bg-transparent"
        let line_gap = 4.0;
        let gap = if self.variant == TabsVariant::Line {
            line_gap
        } else {
            0.0
        };
        let text_px = t.text_px(pick(&trigger, |r| &r.text_size, g).unwrap_or(TextSize::Sm));
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
        let widths: Vec<f32> = galleys
            .iter()
            .map(|g| g.size().x + tpad_l + tpad_r + border_w * 2.0)
            .collect();
        let n = widths.len();
        // base: "inline-flex w-fit items-center justify-center"
        let list_w = widths.iter().sum::<f32>() + gap * n.saturating_sub(1) as f32 + pad_l + pad_r;

        let (list_rect, mut resp) = ui.allocate_exact_size(vec2(list_w, list_h), Sense::hover());
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
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let resps: Vec<Response> = trigger_rects
            .iter()
            .zip(&ids)
            .map(|(r, id)| ui.interact(*r, *id, sense))
            .collect();

        // Selection: a click, or the arrow/Home/End keys on a focused trigger.
        // Only a key moves the focus along: focus is what shows the ring,
        // and a click must not show it.
        let mut next = None;
        let mut by_key = false;
        if enabled {
            for (i, r) in resps.iter().enumerate() {
                if r.clicked() {
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

        // base: "group-data-horizontal/tabs:after:inset-x-0 group-data-horizontal/tabs:after:bottom-[-5px]"
        let below = 5.0;
        // base: "group-data-horizontal/tabs:after:h-0.5"
        let thickness = 2.0;
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
        if ui.is_rect_visible(list_rect.expand(below)) {
            // base: "disabled:pointer-events-none disabled:opacity-50"
            let painter = disabled_painter(ui, self.enabled, 0.5);

            // The list: a muted track in the default variant, nothing in line.
            let list_radius = pick(&list, |r| &r.radius, g)
                .map_or(0.0, |r| t.radius_px(r, list_rect.height() / 2.0));
            let track = (self.variant == TabsVariant::Default).then(|| t.role(Role::Muted));
            // The fade is the triggers' (`disabled:opacity-50`), not the list's.
            ui.painter()
                .extend(box_shapes(list_rect, list_radius, track, None, false));

            for (i, (rect, r)) in trigger_rects.iter().zip(&resps).enumerate() {
                let is_active = i == active;
                let context = State::HORIZONTAL
                    .or(variant_bit)
                    .with(State::SELECTED, is_active);
                let st = interaction_state(&t, r, enabled, context);
                let dark = st.contains(State::DARK);
                let hover = st.contains(State::HOVER);
                let focus = st.contains(State::FOCUS_VISIBLE);
                let line = self.variant == TabsVariant::Line;
                let role = |role, alpha| t.paint(Paint::Role(role, alpha));

                // base: "text-foreground/60 transition-all"
                // base: "hover:text-foreground focus-visible:border-ring"
                // base: "dark:text-muted-foreground dark:hover:text-foreground"
                // base: "data-active:text-foreground"
                let fg = if is_active || hover {
                    role(Role::Foreground, 1.0)
                } else if dark {
                    role(Role::MutedForeground, 1.0)
                } else {
                    // base: "text-foreground/60"
                    role(Role::Foreground, 0.6)
                };
                // base: "group-data-[variant=line]/tabs-list:bg-transparent"
                // base: "data-active:bg-background data-active:text-foreground"
                let bg = match (line, is_active, dark) {
                    (true, _, _) | (false, false, _) => None,
                    (false, true, false) => Some(role(Role::Background, 1.0)),
                    // base: "dark:data-active:bg-input/30"
                    (false, true, true) => Some(role(Role::Input, 0.3)),
                };
                // base: "dark:group-data-[variant=line]/tabs-list:data-active:border-transparent"
                // base: "dark:data-active:border-input"
                let border_role = if is_active && dark {
                    if line {
                        Role::Transparent
                    } else {
                        Role::Input
                    }
                } else if focus {
                    Role::Ring
                } else {
                    Role::Transparent
                };
                let border = pick(&trigger, |r| &r.border_color, st).map(|p| t.paint(p));
                let border = if border_role == Role::Transparent {
                    border
                } else {
                    Some(role(border_role, 1.0))
                };

                let radius = pick(&trigger, |r| &r.radius, st)
                    .map_or(0.0, |r| t.radius_px(r, rect.height() / 2.0));
                if let Some(level) = pick(&trigger, |r| &r.shadow, st) {
                    painter.add(t.shadow(level).as_shape(*rect, radius));
                }
                let stroke = border.map(|b| Stroke::new(border_w, b));
                let mut shapes = box_shapes(*rect, radius, bg, stroke, false);
                if focus {
                    // base: "focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-1 focus-visible:outline-ring"
                    let (ring_w, outline_w) = (3.0, 1.0);
                    // base: "focus-visible:ring-ring/50"
                    let ring_color = t.paint(Paint::Role(Role::Ring, 0.5));
                    shapes.extend(ring_shape(*rect, radius, ring_w, ring_color));
                    shapes.extend(ring_shape(*rect, radius, outline_w, t.role(Role::Ring)));
                }
                if line && is_active {
                    let inner = rect.shrink(border_w);
                    let bar = Rect::from_min_max(
                        pos2(inner.left(), inner.bottom() + below - thickness),
                        pos2(inner.right(), inner.bottom() + below),
                    );
                    shapes.push(Shape::rect_filled(bar, 0.0, role(Role::Foreground, 1.0)));
                }
                painter.extend(shapes);
                let galley = galleys[i].clone();
                painter.galley(rect.center() - galley.size() / 2.0, galley, fg);
            }
        }
        resp
    }
}

/// Renders a tab bar in the default variant. `active` is the selected index.
/// Returns true if it changed.
pub fn tab_bar(ui: &mut Ui, active: &mut usize, labels: &[&str]) -> bool {
    ui.add(Tabs::new(active, labels)).changed()
}
