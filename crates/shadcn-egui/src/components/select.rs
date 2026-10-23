//! shadcn Select, ported from shadcn-svelte `ui/select/*.svelte` at
//! `tools/stylegen/PIN`. Size, padding, radius, colours, ring, shadow and
//! the disabled fade come from the style's
//! `cn-select-trigger`, `cn-select-trigger-icon`, `cn-select-content`,
//! `cn-select-group`, `cn-select-item` and `cn-select-item-indicator`
//! recipes. The chevron and the tick are the theme's `select_trigger` and
//! `check` icons ([`crate::Theme::icons`]).
//!
//! The trigger is a custom-painted box (an `egui::ComboBox` cannot take
//! these metrics) and the list an `egui::Popup` below it. Behaviour follows
//! bits-ui Select: a click, Enter or Space opens the list (so does Up or
//! Down on a focused trigger); Up, Down, Home and End move the highlight,
//! Enter or Space chooses it and closes; Esc and a click outside close
//! without a change; typing a letter jumps to the next option starting with
//! it. The pointer moves the highlight too. An open list takes the keys
//! while its trigger has focus or no widget has; once Tab moves focus to
//! another widget, the keys belong to that widget.
//!
//! Differences from upstream: the options-list API has no groups, labels,
//! separators or per-option disabled state, so `cn-select-label` and
//! `cn-select-separator` are not drawn and the list always sits in one
//! `cn-select-group` (its padding frames the rows). An `egui::ScrollArea`
//! replaces the scroll buttons (`cn-select-scroll-*` are resolved by the
//! style but unused). Each row paints its tick and text in the row's own
//! text colour, which is what the `**:` descendant classes of
//! `cn-select-item` ask for. Without `width`, the trigger is as wide as its
//! widest option (`w-fit` would make it jump with the chosen text); either
//! width is capped to the room (conformance 3.1).

use crate::components::icon::paint_icon;
use crate::components::shared::{
    content_ui, interaction_state, line_box, opacity, part_painter, pressable, ActivationKeys,
    RecipeBox, DEFAULT_SIDE_OFFSET,
};
use crate::style::{pick, Paint, Prop, Recipe, Role, State, TextSize, Weight};
use crate::theme::family_for;
use crate::{layout, Theme};
use egui::{vec2, Color32, Key, Modifiers, Popup, PopupCloseBehavior, Rect, Sense, Ui, Widget};

/// Trigger height variant: shadcn `size="default" | "sm"`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectSize {
    #[default]
    Default,
    Sm,
}

pub struct Select<'a> {
    id: &'a str,
    selected: &'a mut usize,
    options: &'a [&'a str],
    size: SelectSize,
    placeholder: String,
    enabled: bool,
    invalid: bool,
    width: Option<f32>,
    side_offset: f32,
}

impl<'a> Select<'a> {
    /// `selected` indexes `options`; an index past the end shows the placeholder.
    pub fn new(id: &'a str, selected: &'a mut usize, options: &'a [&'a str]) -> Self {
        Self {
            id,
            selected,
            options,
            size: SelectSize::Default,
            placeholder: String::new(),
            enabled: true,
            invalid: false,
            width: None,
            side_offset: DEFAULT_SIDE_OFFSET,
        }
    }
    pub fn size(mut self, size: SelectSize) -> Self {
        self.size = size;
        self
    }
    /// Text shown while `selected` is past the end of the options.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }
    /// `false` draws the select disabled: no hover, no focus, no opening.
    pub fn enabled(mut self, yes: bool) -> Self {
        self.enabled = yes;
        self
    }
    /// shadcn `aria-invalid`: destructive border and ring.
    pub fn invalid(mut self, yes: bool) -> Self {
        self.invalid = yes;
        self
    }
    /// Trigger width in points, capped to the room; default is the widest
    /// option.
    pub fn width(mut self, w: f32) -> Self {
        self.width = Some(w);
        self
    }
    /// Gap between the trigger and the list (shadcn `sideOffset`, default 4).
    pub fn side_offset(mut self, px: f32) -> Self {
        self.side_offset = px;
        self
    }
}

impl Widget for Select<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let t = Theme::current(ui.ctx());
        let ctx = ui.ctx().clone();
        let trig = [&*t.style.select_trigger];
        let enabled = self.enabled && ui.is_enabled();
        let n = self.options.len();
        let has_value = *self.selected < n;
        let size_state = if self.size == SelectSize::Sm {
            State::SIZE_SM
        } else {
            State::SIZE_DEFAULT
        };
        let context = size_state
            .with(State::INVALID, self.invalid)
            .with(State::PLACEHOLDER, !has_value);
        // Geometry depends on neither interaction nor the open state.
        let g = t.state(ui.ctx(), context);
        let get = |f: fn(&Recipe) -> &Prop<f32>| pick(&trig, f, g).unwrap_or(0.0);
        let border_w = get(|r| &r.border_width);
        let (pad_l, pad_r) = (
            get(|r| &r.pad_left) + border_w,
            get(|r| &r.pad_right) + border_w,
        );
        let gap = get(|r| &r.gap);
        // The chevron's own `size-*`, else the trigger's `[&_svg]:size-*`,
        // else the theme's icon size, as a button's.
        let icon = [&*t.style.select_trigger_icon];
        let icon_side = pick(&icon, |r| &r.width, g)
            .or_else(|| pick(&trig, |r| &r.icon_size, g))
            .unwrap_or(t.metrics.icon);
        let icon_h = pick(&icon, |r| &r.height, g).unwrap_or(icon_side);
        let weight = pick(&trig, |r| &r.weight, g).unwrap_or(Weight::Normal);
        let text_size = pick(&trig, |r| &r.text_size, g).unwrap_or(TextSize::Sm);
        let font = egui::FontId::new(t.text_px(text_size), family_for(&ctx, weight));
        let painter = ui.painter().clone();
        let shown = if has_value {
            self.options[*self.selected]
        } else {
            self.placeholder.as_str()
        };
        let galley = painter.layout_no_wrap(shown.to_owned(), font.clone(), Color32::PLACEHOLDER);
        let fit = self
            .options
            .iter()
            .copied()
            .chain(std::iter::once(self.placeholder.as_str()))
            .map(|o| {
                painter
                    .layout_no_wrap(o.to_owned(), font.clone(), Color32::PLACEHOLDER)
                    .size()
                    .x
            })
            .fold(0.0, f32::max);
        // `w-fit`, capped to the room, raised to the padding, gap and
        // chevron; the text area shrinks and clips (conformance 3.1).
        let chrome = gap + icon_side + pad_l + pad_r;
        let width = layout::capped_width(ui, self.width.unwrap_or(fit + chrome), chrome);
        // No height: CSS's initial `auto`, one line of text in the padding box.
        let height = pick(&trig, |r| &r.height, g).unwrap_or_else(|| {
            line_box(&t, &trig, g, text_size)
                .max(galley.size().y)
                .max(icon_h)
                + get(|r| &r.pad_top)
                + get(|r| &r.pad_bottom)
                + border_w * 2.0
        });

        // Overflowing at its content minimum, the trigger keeps its parent's
        // width (conformance 3.6); otherwise `allocate_space` takes a
        // justified layout's width, as egui's `Button` does.
        let rect = if width > ui.available_width() {
            layout::allocate_overflowing(ui, vec2(width, height), Sense::hover()).0
        } else {
            ui.allocate_space(vec2(width, height)).1
        };
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut resp = ui.interact(
            rect,
            ui.make_persistent_id(("shadcn_select", self.id)),
            sense,
        );
        // bits' trigger opens on Enter and Space, both on press.
        pressable(&mut resp, ActivationKeys::EnterAndSpace);
        let popup_id = Popup::default_response_id(&resp);
        let hl_id = popup_id.with("highlight");
        let was_open = Popup::is_id_open(&ctx, popup_id);

        // Keyboard: the trigger keeps focus while the list is open, so the
        // arrows are claimed here, not left to egui's focus movement. A press
        // focuses the trigger (`pressable`), but a click on the open list's
        // padding or scroll bar makes egui drop that focus without giving it
        // to another widget; the list keeps the keys then, as bits' content
        // does while open. Once Tab moves focus to another widget, the keys
        // are that widget's.
        let mut moved = false;
        let mut chosen = None;
        let focused = resp.has_focus();
        let keys_to_list = was_open && (focused || ctx.memory(|m| m.focused()).is_none());
        if enabled && (focused || keys_to_list) {
            ctx.memory_mut(|m| {
                m.set_focus_lock_filter(
                    resp.id,
                    egui::EventFilter {
                        vertical_arrows: true,
                        ..Default::default()
                    },
                )
            });
        }
        let activation = ui.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space));
        let mut toggle = resp.clicked();
        if enabled && keys_to_list && n > 0 {
            let mut hl = ctx
                .data(|d| d.get_temp::<usize>(hl_id))
                .unwrap_or(if has_value { *self.selected } else { 0 })
                .min(n - 1);
            let key = |k: Key| ui.input_mut(|i| i.consume_key(Modifiers::NONE, k));
            if key(Key::ArrowDown) {
                hl = (hl + 1).min(n - 1);
                moved = true;
            }
            if key(Key::ArrowUp) {
                hl = hl.saturating_sub(1);
                moved = true;
            }
            if key(Key::Home) {
                hl = 0;
                moved = true;
            }
            if key(Key::End) {
                hl = n - 1;
                moved = true;
            }
            let typed: Vec<char> = ui.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::Text(s) => s.chars().next().filter(|c| !c.is_whitespace()),
                        _ => None,
                    })
                    .collect()
            });
            for c in typed {
                let starts = |o: &&str| {
                    o.chars()
                        .next()
                        .is_some_and(|f| f.to_lowercase().eq(c.to_lowercase()))
                };
                let after = (1..=n)
                    .map(|d| (hl + d) % n)
                    .find(|&i| starts(&self.options[i]));
                if let Some(i) = after {
                    hl = i;
                    moved = true;
                }
            }
            if activation {
                chosen = Some(hl);
                toggle = false;
            }
            ctx.data_mut(|d| d.insert_temp(hl_id, hl));
        } else if enabled
            && !was_open
            && focused
            && n > 0
            && ui.input_mut(|i| {
                i.consume_key(Modifiers::NONE, Key::ArrowDown)
                    | i.consume_key(Modifiers::NONE, Key::ArrowUp)
            })
        {
            Popup::open_id(&ctx, popup_id);
        }
        if let Some(i) = chosen {
            if *self.selected != i {
                *self.selected = i;
                resp.mark_changed();
            }
            Popup::close_id(&ctx, popup_id);
        }
        if toggle && enabled {
            Popup::toggle_id(&ctx, popup_id);
        }
        let open = Popup::is_id_open(&ctx, popup_id) && enabled;

        // The trigger.
        let st = interaction_state(
            &t,
            &resp,
            context
                .with(State::EXPANDED, open)
                .with(State::DISABLED, !enabled),
        );
        if ui.is_rect_visible(rect) {
            let painter = part_painter(ui, &[&trig[..]], st);
            let paint = |f: fn(&Recipe) -> &Prop<Paint>| pick(&trig, f, st).map(|p| t.paint(p));
            RecipeBox::resolve(&t, &trig, st).paint(&painter, rect);

            let icon_rect = Rect::from_center_size(
                egui::pos2(rect.right() - pad_r - icon_side / 2.0, rect.center().y),
                vec2(icon_side, icon_h),
            );
            let icon_color = pick(&icon, |r| &r.text, st).map(|p| t.paint(p));
            if let Some(c) = icon_color {
                // The chevron sits in the trigger: both opacities apply.
                paint_icon(
                    &part_painter(ui, &[&icon[..], &trig[..]], st),
                    icon_rect,
                    &t.icons.select_trigger,
                    c,
                    t.metrics.icon_stroke,
                );
            }
            let inherited = t.role(Role::Foreground); // behaviour-ok: inherits Foreground; upstream sets no text colour
            let text_color = paint(|r| &r.text).unwrap_or(inherited);
            let text_area = Rect::from_min_max(
                egui::pos2(rect.left() + pad_l, rect.top()),
                egui::pos2(icon_rect.left() - gap, rect.bottom()),
            );
            let at = egui::pos2(text_area.left(), rect.center().y - galley.size().y / 2.0);
            painter
                .with_clip_rect(text_area)
                .galley(at, galley, text_color);
        }
        // The shown text is the combobox's value; its name comes from
        // `labelled_by(label.id)`, which an own label would override.
        resp.widget_info(|| egui::WidgetInfo {
            enabled,
            current_text_value: Some(shown.to_owned()),
            ..egui::WidgetInfo::new(egui::WidgetType::ComboBox)
        });

        // The list.
        if open {
            // Menu content draws with the menu's colours (upstream
            // `menuColor`): an inverted menu is dark, `dark:` rules included,
            // also in light mode. The trigger above keeps the page theme.
            let menu = t.menu_theme();
            let t = &menu;
            let l = [&*t.style.select_content];
            let item = [&*t.style.select_item];
            let group = [&*t.style.select_group];
            let gs = t.state(ui.ctx(), State::NONE);
            let ist = t.state(ui.ctx(), State::FOCUS.with(State::SELECTED, false));
            let px = |l: &[&Recipe], f: fn(&Recipe) -> &Prop<f32>| pick(l, f, gs).unwrap_or(0.0);
            let (gpt, gpr, gpb, gpl) = (
                px(&group, |r| &r.pad_top),
                px(&group, |r| &r.pad_right),
                px(&group, |r| &r.pad_bottom),
                px(&group, |r| &r.pad_left),
            );
            // The frame paints outside the list's `Ui`: fade its colours here.
            // Inside, the viewport sits in the content and the group in the
            // viewport; neither paints anything of its own.
            let alpha = opacity(&[&l[..]], gs);
            let viewport = [&*t.style.select_viewport];
            let panel = RecipeBox::resolve(t, &l, gs).faded(alpha);
            let frame = panel.frame([gpt, gpr, gpb, gpl]);
            let item_font = egui::FontId::new(
                t.text_px(pick(&item, |r| &r.text_size, ist).unwrap_or(TextSize::Sm)),
                family_for(
                    &ctx,
                    pick(&item, |r| &r.weight, ist).unwrap_or(Weight::Normal),
                ),
            );
            let item_size = pick(&item, |r| &r.text_size, ist).unwrap_or(TextSize::Sm);
            let line = pick(&item, |r| &r.line_height, ist)
                .map_or_else(|| t.line_px(item_size), |k| k * t.text_px(item_size));
            let mut galleys: Vec<_> = self
                .options
                .iter()
                .map(|o| {
                    painter.layout_no_wrap((*o).to_owned(), item_font.clone(), Color32::PLACEHOLDER)
                })
                .collect();
            let (ipl, ipr) = (px(&item, |r| &r.pad_left), px(&item, |r| &r.pad_right));
            let (ipt, ipb) = (px(&item, |r| &r.pad_top), px(&item, |r| &r.pad_bottom));
            let min_row = px(&item, |r| &r.min_height);
            let needed = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max) + ipl + ipr;
            let side_offset = self.side_offset;
            let min_w = px(&l, |r| &r.min_width).max(rect.width());
            // The list is `w-fit` with `min-w-*` and the trigger's width,
            // capped to the window (conformance 3.1); the content minimum
            // is the group's and an item's padding.
            let [_, br, _, bl] = panel.border_widths();
            let chrome = gpl + gpr + br + bl;
            let outer_w =
                layout::overlay_width(&ctx, min_w.max(needed + chrome), 0.0, chrome + ipl + ipr);
            let inner_w = outer_w - chrome;
            // Capped, an item's text wraps in its row, as the browser wraps it.
            let text_w = inner_w - ipl - ipr;
            for (galley, o) in galleys.iter_mut().zip(self.options) {
                if galley.size().x > text_w {
                    *galley = painter.layout(
                        (*o).to_owned(),
                        item_font.clone(),
                        Color32::PLACEHOLDER,
                        text_w,
                    );
                }
            }
            let screen = ctx.content_rect();
            let max_h = (screen.bottom() - rect.bottom()).max(rect.top() - screen.top())
                - side_offset
                - gpt
                - gpb;
            let hl = ctx.data(|d| d.get_temp::<usize>(hl_id));
            let hl_missing = hl.is_none();
            let hl = hl.unwrap_or(if has_value { *self.selected } else { 0 });
            ctx.data_mut(|d| d.insert_temp(hl_id, hl));
            let pointer_moved = ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
            let current = *self.selected;
            let mut hover_hl = None;
            let mut clicked = None;
            let list = Popup::from_response(&resp)
                .open_memory(None)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .gap(side_offset)
                .frame(frame)
                .show(|ui| {
                    let slot = RecipeBox::reserve(ui);
                    ui.set_width(inner_w);
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    // The rows sit in the group, the group in the viewport,
                    // the viewport in the content.
                    let chain = [&group[..], &viewport[..], &l[..]];
                    content_ui(ui, &chain, gs, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(max_h.max(0.0))
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                for (i, galley) in galleys.iter().enumerate() {
                                    let (_, row) = ui.allocate_space(vec2(
                                        inner_w,
                                        (galley.size().y.max(line) + ipt + ipb).max(min_row),
                                    ));
                                    let r =
                                        ui.interact(row, resp.id.with(("item", i)), Sense::click());
                                    if r.hovered() && pointer_moved {
                                        hover_hl = Some(i);
                                    }
                                    if r.clicked() {
                                        clicked = Some(i);
                                    }
                                    let highlighted = hl == i;
                                    let state = State::NONE
                                        .with(State::FOCUS, highlighted)
                                        .with(State::SELECTED, i == current);
                                    let s = t.state(ui.ctx(), state);
                                    if highlighted && (moved || hl_missing) {
                                        r.scroll_to_me(None);
                                    }
                                    if ui.is_rect_visible(row) {
                                        // The row's text and indicator sit in the row.
                                        let painter = part_painter(ui, &[&item[..]], s);
                                        RecipeBox::resolve(t, &item, s).paint(&painter, row);
                                        // The row's own text colour serves tick and text alike.
                                        let inherited = t.role(Role::PopoverForeground); // behaviour-ok: inherits PopoverForeground from the content
                                        let color = pick(&item, |r| &r.text, s)
                                            .or(pick(&l, |r| &r.text, s))
                                            .map_or(inherited, |p| t.paint(p));
                                        let text_painter = part_painter(
                                            ui,
                                            &[&[&*t.style.select_item_text][..], &item[..]],
                                            s,
                                        );
                                        text_painter.galley(
                                            egui::pos2(
                                                row.left() + ipl,
                                                row.center().y - galley.size().y / 2.0,
                                            ),
                                            galley.clone(),
                                            color,
                                        );
                                        if i == current {
                                            let ind = [&*t.style.select_item_indicator];
                                            // No size: CSS's initial `auto`, the
                                            // tick at the row's `[&_svg]:size-*`.
                                            let tick_side = pick(&item, |r| &r.icon_size, s)
                                                .unwrap_or(t.metrics.icon);
                                            let (w, h) = (
                                                pick(&ind, |r| &r.width, s).unwrap_or(tick_side),
                                                pick(&ind, |r| &r.height, s).unwrap_or(tick_side),
                                            );
                                            let right =
                                                pick(&ind, |r| &r.inset_right, s).unwrap_or(0.0);
                                            let tick = Rect::from_center_size(
                                                egui::pos2(
                                                    row.right() - right - w / 2.0,
                                                    row.center().y,
                                                ),
                                                vec2(w, h),
                                            );
                                            // The icon sits in the indicator.
                                            let tick_painter = part_painter(
                                                ui,
                                                &[
                                                    &[&*t.style.select_item_indicator_icon][..],
                                                    &ind[..],
                                                    &item[..],
                                                ],
                                                s,
                                            );
                                            paint_icon(
                                                &tick_painter,
                                                tick,
                                                &t.icons.check,
                                                color,
                                                t.metrics.icon_stroke,
                                            );
                                        }
                                    }
                                    r.widget_info(|| {
                                        egui::WidgetInfo::selected(
                                            egui::WidgetType::SelectableLabel,
                                            true,
                                            i == current,
                                            self.options[i],
                                        )
                                    });
                                }
                            });
                    });
                    slot
                });
            if let Some(i) = hover_hl.filter(|&i| i != hl) {
                ctx.data_mut(|d| d.insert_temp(hl_id, i));
                ctx.request_repaint();
            }
            if let Some(i) = clicked {
                if *self.selected != i {
                    *self.selected = i;
                    resp.mark_changed();
                }
                Popup::close_id(&ctx, popup_id);
            }
            if let Some(list) = list {
                panel.paint_reserved(&ctx, list.response.layer_id, list.inner, list.response.rect);
            }
        }
        if !Popup::is_id_open(&ctx, popup_id) {
            ctx.data_mut(|d| d.remove_temp::<usize>(hl_id));
        }
        resp
    }
}

/// Renders a select. `selected` is the chosen index. Returns true if it changed.
pub fn select(ui: &mut Ui, id: &str, selected: &mut usize, options: &[&str]) -> bool {
    ui.add(Select::new(id, selected, options)).changed()
}
