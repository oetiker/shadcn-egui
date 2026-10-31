//! shadcn Scroll Area, ported from shadcn-svelte
//! `ui/scroll-area/scroll-area.svelte` and `scroll-area-scrollbar.svelte`
//! at `tools/stylegen/PIN`, which wrap the bits-ui scroll area with its
//! default `type="hover"`. The bars and the thumb come from the style's
//! `cn-scroll-area-scrollbar` and `cn-scroll-area-thumb` recipes, the
//! focus ring from `cn-scroll-area-viewport`. egui's
//! [`egui::ScrollArea`] scrolls the content; its own bars stay hidden, and
//! this module paints shadcn's over the content.
//!
//! Behaviour follows bits-ui: an axis shows its bar while the pointer is
//! over the area and that axis overflows, and hides it 600 ms after the
//! pointer leaves, with no fade. The bars take no room from the content.
//! Pressing the thumb drags it from where it was taken; pressing the
//! track centres the thumb on the pointer, then drags. The wheel scrolls
//! over the bars too; a touch screen scrolls by dragging the content, a
//! mouse does not.
//!
//! Keyboard follows Chrome's focusable scrollers: an area that overflows
//! and holds no focusable widget is a Tab stop and shows the viewport's
//! focus ring; focused, the arrow keys scroll it by 40 px, Page Up/Down
//! and Space (Shift+Space up) by 87.5 % of its height, Home and End to
//! the start and the end. A widget inside that takes focus is scrolled
//! into view, centred if it was not fully visible, as the browser scrolls
//! a focused element.
//!
//! Differences from upstream:
//! - The content is clipped to a rectangle, not to the root's radius
//!   (`rounded-[inherit]`): egui clips to rectangles only. The focus ring
//!   has square corners for the same reason. Card clips its content the
//!   same way.
//! - The pointer counts as over the area inside the viewport; the caller's
//!   border around it (a 1 px frame) is outside, where the browser counts
//!   the root's border in.
//! - Key scrolling jumps where Chrome animates it.
//! - bits-ui's other visibility types (`scroll`, `auto`, `always`) and
//!   `scrollHideDelay` are not offered yet.

use crate::components::shared::{interaction_state, part_painter, RecipeBox};
use crate::style::app::default_variant;
use crate::style::{pick_len, Recipe, State};
use crate::Theme;
use egui::scroll_area::{ScrollAreaOutput, ScrollBarVisibility, ScrollSource};
use egui::{Id, Key, Margin, Modifiers, Rect, Sense, Ui, Vec2, Vec2b};

default_variant!(
    /// The scroll area's variant: upstream has none, so only app variants
    /// ([`ScrollAreaVariant::app`]) differ from `Default`.
    ScrollAreaVariant,
    /// The scroll area's recipes an app variant's edit changes.
    ScrollAreaRecipes {
        /// `data-slot="scroll-area"`: the root.
        scroll_area,
        /// `data-slot="scroll-area-viewport"`: the scrolled box and its
        /// focus ring.
        scroll_area_viewport,
        /// `data-slot="scroll-area-scrollbar"`: a bar, the thumb's track.
        scroll_area_scrollbar,
        /// `data-slot="scroll-area-thumb"`: the thumb.
        scroll_area_thumb,
    }
);

/// bits-ui's `scrollHideDelay` default, in seconds.
const HIDE_DELAY: f64 = 0.6; // behaviour-ok: bits-ui ScrollArea `scrollHideDelay` default (600 ms)
/// bits-ui's smallest thumb length (`getThumbSize`).
const MIN_THUMB: f32 = 18.0; // behaviour-ok: bits-ui `getThumbSize` minimum
/// Chrome's arrow-key scroll step (`kPixelsPerLineStep`).
const LINE_STEP: f32 = 40.0; // behaviour-ok: Chrome's arrow-key scroll step
/// Chrome's page step as a share of the visible length
/// (`ScrollableArea::PageStep`).
const PAGE_SHARE: f32 = 0.875; // behaviour-ok: Chrome's page step, 87.5 % of the visible length

/// A scroll area with shadcn's bars: `ScrollArea::vertical().show(ui, |ui|
/// ..)`, as egui's [`egui::ScrollArea`], whose name and setters it keeps.
/// Upstream's `orientation` is the constructor: [`ScrollArea::vertical`]
/// (upstream's default), [`ScrollArea::horizontal`] or
/// [`ScrollArea::both`].
///
/// The area is as large as egui's would be: the room the `Ui` has, capped
/// by [`ScrollArea::max_width`] and [`ScrollArea::max_height`], and
/// shrunk to the content where [`ScrollArea::auto_shrink`] says so. The
/// size and border of upstream's root come from the caller, as an
/// [`egui::Frame`] around the area.
///
/// ```
/// use shadcn_egui::components::scroll_area::ScrollArea;
/// use shadcn_egui::components::separator::separator;
///
/// fn tags(ui: &mut egui::Ui) {
///     ScrollArea::vertical().max_height(288.0).show(ui, |ui| {
///         for i in (1..=50).rev() {
///             ui.label(format!("v1.2.0-beta.{i}"));
///             separator(ui);
///         }
///     });
/// }
/// ```
#[derive(Clone, Debug)]
#[must_use = "You should call .show()"]
pub struct ScrollArea {
    area: egui::ScrollArea,
    direction: Vec2b,
    /// The id salt as egui hashes it.
    salt: Id,
    stick_to_bottom: bool,
    /// As given to egui's `auto_shrink`.
    shrink: Vec2b,
    variant: ScrollAreaVariant,
}

/// What the area keeps between passes, in egui's memory.
#[derive(Clone, Copy, Debug, Default)]
struct Memory {
    /// The input time at which the pointer was last over the area.
    hovered_at: Option<f64>,
    /// Per axis, while a bar drag goes on: where on the thumb it was taken,
    /// from the thumb's start; `None` takes it at the centre.
    grab: [Option<f32>; 2],
    /// This area moved the offset in the last pass: egui's stick-to-end
    /// must not snap it back in this one.
    moved: bool,
    /// The widget inside that had focus at the end of the last pass.
    focused: Option<Id>,
}

/// What the content held, read at its end inside the egui area.
#[derive(Debug, Default)]
struct Scan {
    /// The widget inside that had focus in the last pass.
    before: Option<Id>,
    /// Whether an enabled widget inside can take focus.
    focusable: bool,
    /// The widget inside that has focus.
    focused: Option<Id>,
    /// The caller's `clip_rect_margin` and scroll fade, which the content
    /// gets back.
    clip_margin: f32,
    fade: egui::style::ScrollFadeStyle,
    /// The axes the area scrolls.
    direction: Vec2b,
}

impl Scan {
    /// Gives the content the caller's look back, and counts the widgets
    /// the `Ui`'s layer holds before the content.
    fn start(&self, ui: &mut Ui) -> usize {
        ui.visuals_mut().clip_rect_margin = self.clip_margin;
        ui.spacing_mut().scroll.fade = self.fade;
        // The viewport clips the axis it does not scroll too (bits'
        // `overflow-x: hidden` on a vertical area); egui clips only the
        // scrolled axes.
        let (mut clip, max) = (ui.clip_rect(), ui.max_rect());
        for d in 0..2 {
            if !self.direction[d] {
                clip.min[d] = clip.min[d].max(max.min[d]);
                clip.max[d] = clip.max[d].min(max.max[d]);
            }
        }
        ui.set_clip_rect(clip);
        let layer = ui.layer_id();
        ui.ctx()
            .viewport(|v| v.this_pass.widgets.get_layer(layer).count())
    }

    /// Reads the widgets the content added after `start`, and scrolls a
    /// widget that took focus into view.
    fn end(&mut self, ui: &Ui, start: usize) {
        let layer = ui.layer_id();
        let focused = ui.memory(|m| m.focused());
        let (focusable, focused_rect) = ui.ctx().viewport(|v| {
            let mut focusable = false;
            let mut rect = None;
            for w in v.this_pass.widgets.get_layer(layer).skip(start) {
                focusable |= w.enabled && w.sense.is_focusable();
                if Some(w.id) == focused {
                    rect = Some(w.rect);
                }
            }
            (focusable, rect)
        });
        self.focusable = focusable;
        self.focused = focused.filter(|_| focused_rect.is_some());
        // The browser scrolls an element that takes focus into view,
        // centred if it is not fully visible (Blink's `CenterIfNeeded`),
        // at once: a focus scroll is not smooth.
        if let Some(rect) = focused_rect {
            if self.focused != self.before && !ui.clip_rect().contains_rect(rect) {
                ui.scroll_to_rect_animation(
                    rect,
                    Some(egui::Align::Center),
                    egui::style::ScrollAnimation::none(),
                );
            }
        }
    }
}

impl ScrollArea {
    fn new(direction: Vec2b) -> Self {
        Self {
            area: egui::ScrollArea::new(direction),
            direction,
            salt: Id::new("scroll_area"),
            stick_to_bottom: false,
            shrink: Vec2b::TRUE,
            variant: ScrollAreaVariant::Default,
        }
    }

    /// Scrolls up and down: upstream's default `orientation="vertical"`.
    #[inline]
    pub fn vertical() -> Self {
        Self::new(Vec2b::new(false, true))
    }

    /// Scrolls left and right: `orientation="horizontal"`.
    #[inline]
    pub fn horizontal() -> Self {
        Self::new(Vec2b::new(true, false))
    }

    /// Scrolls both ways: `orientation="both"`.
    #[inline]
    pub fn both() -> Self {
        Self::new(Vec2b::TRUE)
    }

    /// The widest the area grows, as egui's `max_width`.
    #[inline]
    pub fn max_width(mut self, width: f32) -> Self {
        self.area = self.area.max_width(width);
        self
    }

    /// The tallest the area grows, as egui's `max_height`.
    #[inline]
    pub fn max_height(mut self, height: f32) -> Self {
        self.area = self.area.max_height(height);
        self
    }

    /// Per axis, whether the area shrinks to content smaller than the room
    /// it has, as egui's `auto_shrink` (default: both).
    #[inline]
    pub fn auto_shrink(mut self, shrink: impl Into<Vec2b>) -> Self {
        self.shrink = shrink.into();
        self.area = self.area.auto_shrink(self.shrink);
        self
    }

    /// Keeps the area scrolled to the bottom while new content comes in,
    /// as egui's `stick_to_bottom`: scrolling up lets go, scrolling back
    /// to the bottom sticks again.
    #[inline]
    pub fn stick_to_bottom(mut self, stick: bool) -> Self {
        self.stick_to_bottom = stick;
        self
    }

    /// A source for the area's id where two areas share a `Ui`, as egui's
    /// `id_salt`; the default salt is `"scroll_area"`, egui's.
    #[inline]
    pub fn id_salt(mut self, salt: impl std::hash::Hash) -> Self {
        // `Id::new` hashes `&salt` as `salt`: egui gets the same id.
        self.salt = Id::new(&salt);
        self.area = self.area.id_salt(salt);
        self
    }

    /// An app variant ([`ScrollAreaVariant::app`]).
    #[inline]
    pub fn variant(mut self, v: impl Into<ScrollAreaVariant>) -> Self {
        self.variant = v.into();
        self
    }

    /// Shows the area with `add_contents` in it, as egui's `show`.
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> ScrollAreaOutput<R> {
        self.run(ui, |area, ui, scan| {
            area.show(ui, |ui| {
                let start = scan.start(ui);
                let inner = add_contents(ui);
                scan.end(ui, start);
                inner
            })
        })
    }

    /// Shows only the rows of `total_rows` (each `row_height_sans_spacing`
    /// tall, plus the item spacing) that are in view, as egui's
    /// `show_rows`: `add_contents` gets their range.
    pub fn show_rows<R>(
        self,
        ui: &mut Ui,
        row_height_sans_spacing: f32,
        total_rows: usize,
        add_contents: impl FnOnce(&mut Ui, std::ops::Range<usize>) -> R,
    ) -> ScrollAreaOutput<R> {
        self.run(ui, |area, ui, scan| {
            area.show_rows(ui, row_height_sans_spacing, total_rows, |ui, rows| {
                let start = scan.start(ui);
                let inner = add_contents(ui, rows);
                scan.end(ui, start);
                inner
            })
        })
    }

    fn run<R>(
        self,
        ui: &mut Ui,
        show: impl FnOnce(egui::ScrollArea, &mut Ui, &mut Scan) -> ScrollAreaOutput<R>,
    ) -> ScrollAreaOutput<R> {
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.salt);
        let mem_id = id.with("shadcn_egui::scroll_area");
        let mut mem: Memory = ctx.data(|d| d.get_temp(mem_id)).unwrap_or_default();
        let area = self
            .area
            // An offset this area set in the last pass is the user's
            // scroll, which egui's stick-to-end would undo.
            .stick_to_bottom(self.stick_to_bottom && !mem.moved)
            // bits' bars lie over the viewport and take no room.
            .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
            .content_margin(Margin::ZERO)
            // The browser scrolls by dragging the content only on touch.
            .scroll_source(ScrollSource {
                scroll_bar: false,
                drag: ctx.input(|i| i.has_touch_screen()),
                mouse_wheel: true,
            });
        let mut scan = Scan {
            before: mem.focused,
            clip_margin: ui.visuals().clip_rect_margin,
            fade: ui.spacing().scroll.fade,
            direction: self.direction,
            ..Scan::default()
        };
        // Two egui extras upstream lacks: content painted 3 px past the
        // viewport (`clip_rect_margin`), where `overflow: hidden` clips at
        // its edge, and fades over a scrolled edge.
        ui.visuals_mut().clip_rect_margin = 0.0;
        ui.spacing_mut().scroll.fade.strength = 0.0;
        let mut out = show(area, ui, &mut scan);
        ui.visuals_mut().clip_rect_margin = scan.clip_margin;
        ui.spacing_mut().scroll.fade = scan.fade;
        debug_assert_eq!(out.id, id, "the crate's area id is egui's");
        mem.focused = scan.focused;

        let t = Theme::current(&ctx);
        let v = t.resolve(self.variant);
        let bar = [v.part(
            &t.style,
            |_, s| &s.scroll_area_scrollbar,
            |r| &r.scroll_area_scrollbar,
        )];
        let thumb = [v.part(
            &t.style,
            |_, s| &s.scroll_area_thumb,
            |r| &r.scroll_area_thumb,
        )];
        let viewport = [v.part(
            &t.style,
            |_, s| &s.scroll_area_viewport,
            |r| &r.scroll_area_viewport,
        )];
        // egui's `inner_rect` is the room before the area shrinks to
        // its content; the viewport is what it allocates in the end.
        let inner = {
            let mut size = out.inner_rect.size();
            for d in 0..2 {
                size[d] = match (self.direction[d], self.shrink[d]) {
                    (true, true) => size[d].min(out.content_size[d]),
                    (true, false) => size[d],
                    (false, true) => out.content_size[d],
                    (false, false) => size[d].max(out.content_size[d]),
                };
            }
            Rect::from_min_size(out.inner_rect.min, size)
        };
        let max_scroll = (out.content_size - inner.size()).max(Vec2::ZERO);
        // bits compares the viewport's whole-pixel `offsetHeight` with its
        // `scrollHeight`: sub-pixel overflow shows no bar.
        let overflows =
            |d: usize| self.direction[d] && out.content_size[d].round() > inner.size()[d].round();
        let overflow = [overflows(0), overflows(1)];
        let mut offset = out.state.offset;

        // Keyboard: a Tab stop where the area overflows and holds nothing
        // focusable (Chrome's focusable scrollers).
        let focus_id = id.with("shadcn_egui::scroll_area::viewport");
        if (overflow[0] || overflow[1]) && !scan.focusable {
            let resp = ui.interact(inner, focus_id, Sense::FOCUSABLE);
            ctx.accesskit_node_builder(resp.id, |node| {
                node.set_role(egui::accesskit::Role::ScrollView);
            });
            if resp.has_focus() {
                ctx.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        focus_id,
                        egui::EventFilter {
                            horizontal_arrows: self.direction.x,
                            vertical_arrows: self.direction.y,
                            ..Default::default()
                        },
                    )
                });
                let page = inner.height() * PAGE_SHARE;
                let keys = ctx.input_mut(|i| {
                    let mut n = |m: Modifiers, k: Key| i.count_and_consume_key(m, k) as f32;
                    let none = Modifiers::NONE;
                    // A pattern without Shift matches a shifted key too:
                    // Shift+Space is taken first.
                    let shift_space = n(Modifiers::SHIFT, Key::Space);
                    [
                        (n(none, Key::ArrowRight) - n(none, Key::ArrowLeft)) * LINE_STEP,
                        (n(none, Key::ArrowDown) - n(none, Key::ArrowUp)) * LINE_STEP
                            + (n(none, Key::PageDown) + n(none, Key::Space)
                                - n(none, Key::PageUp)
                                - shift_space)
                                * page,
                        n(none, Key::Home),
                        n(none, Key::End),
                    ]
                });
                if self.direction.x {
                    offset.x += keys[0];
                }
                if self.direction.y {
                    offset.y += keys[1];
                    if keys[2] > 0.0 {
                        offset.y = 0.0;
                    }
                    if keys[3] > 0.0 {
                        offset.y = max_scroll.y;
                    }
                }
            }
            let st = interaction_state(&t, &resp, v.bits());
            RecipeBox::resolve(&ctx, &t, &viewport, st)
                .transitioned(&ctx, focus_id, &viewport, st)
                .paint(&part_painter(ui, focus_id, &[&viewport[..]], st), inner);
        }

        // Visibility: bits' `type="hover"`.
        let now = ctx.input(|i| i.time);
        let bar_id = |d: usize| id.with(("shadcn_egui::scroll_area::bar", d));
        let dragging = ctx.is_being_dragged(bar_id(0)) || ctx.is_being_dragged(bar_id(1));
        if ui.rect_contains_pointer(inner) {
            mem.hovered_at = Some(now);
        }
        let since = mem.hovered_at.map_or(f64::INFINITY, |at| now - at);
        if since > 0.0 && since < HIDE_DELAY {
            ctx.request_repaint_after_secs((HIDE_DELAY - since) as f32);
        }
        let hover = since < HIDE_DELAY || dragging;
        let shown = [overflow[0] && hover, overflow[1] && hover];

        let states = [
            t.state(&ctx, v.bits().or(State::HORIZONTAL)),
            t.state(&ctx, v.bits().or(State::VERTICAL)),
        ];
        // The bar across each axis: the horizontal bar's height, the
        // vertical bar's width.
        let thickness = [
            pick_len(&bar, |r| &r.height, states[0]),
            pick_len(&bar, |r| &r.width, states[1]),
        ];
        let [bar_height, bar_width] = thickness;
        for d in 0..2 {
            if !shown[d] {
                mem.grab[d] = None;
                continue;
            }
            let st = states[d];
            // bits' corner: each bar stops short of the other while both
            // show.
            let other = if shown[1 - d] { thickness[1 - d] } else { 0.0 };
            let bar_rect = if d == 1 {
                Rect::from_min_max(
                    egui::pos2(inner.right() - bar_width, inner.top()),
                    egui::pos2(inner.right(), inner.bottom() - other),
                )
            } else {
                Rect::from_min_max(
                    egui::pos2(inner.left(), inner.bottom() - bar_height),
                    egui::pos2(inner.right() - other, inner.bottom()),
                )
            };
            let bar_box = RecipeBox::resolve(&ctx, &t, &bar, st);
            let track = track_rect(&bar_box, &bar, st, bar_rect);
            let len = track.size()[d];
            let thumb_len = (len * inner.size()[d] / out.content_size[d]).max(MIN_THUMB);
            let free = (len - thumb_len).max(0.0);
            let thumb_start = |offset: f32| {
                if max_scroll[d] > 0.0 {
                    track.min[d] + offset.clamp(0.0, max_scroll[d]) / max_scroll[d] * free
                } else {
                    track.min[d]
                }
            };

            // Interacted after the content, so it lies above it, as bits'
            // absolute bar does. A press drags at once (no click sense).
            let resp = ui.interact(bar_rect, bar_id(d), Sense::DRAG);
            if let Some(p) = resp.interact_pointer_pos() {
                if resp.drag_started() {
                    let start = thumb_start(offset[d]);
                    let at = p[d] - start;
                    // bits' `pointerOffset || thumbCenter`.
                    mem.grab[d] = (at > 0.0 && at <= thumb_len).then_some(at);
                }
                if resp.dragged() {
                    let grab = mem.grab[d].unwrap_or(thumb_len / 2.0);
                    // bits' `getScrollPositionFromPointer`.
                    if free > 0.0 {
                        offset[d] = (p[d] - track.min[d] - grab) / free * max_scroll[d];
                    }
                }
            }
            if !resp.dragged() && !resp.drag_started() {
                mem.grab[d] = None;
            }

            if ui.is_rect_visible(bar_rect) {
                bar_box.paint(&part_painter(ui, bar_id(d), &[&bar[..]], st), bar_rect);
                let start = thumb_start(offset[d]);
                let mut thumb_rect = track;
                thumb_rect.min[d] = start;
                thumb_rect.max[d] = start + thumb_len;
                let thumb_id = bar_id(d).with("thumb");
                RecipeBox::resolve(&ctx, &t, &thumb, st).paint(
                    &part_painter(ui, thumb_id, &[&thumb[..], &bar[..]], st),
                    thumb_rect,
                );
            }
        }

        let offset = offset.clamp(Vec2::ZERO, max_scroll);
        mem.moved = offset != out.state.offset;
        if mem.moved {
            out.state.offset = offset;
            out.state.store(&ctx, out.id);
            ctx.request_repaint();
        }
        ctx.data_mut(|d| d.insert_temp(mem_id, mem));
        out
    }
}

/// The bar's content box: `rect` less its border and padding, where the
/// thumb runs.
fn track_rect(bar_box: &RecipeBox, bar: &[&Recipe], st: State, rect: Rect) -> Rect {
    let [bt, br, bb, bl] = bar_box.border_widths();
    let pad = |f: fn(&Recipe) -> &crate::style::Prop<f32>| pick_len(bar, f, st);
    Rect::from_min_max(
        rect.min + egui::vec2(bl + pad(|r| &r.pad_left), bt + pad(|r| &r.pad_top)),
        rect.max - egui::vec2(br + pad(|r| &r.pad_right), bb + pad(|r| &r.pad_bottom)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{AppVariant, Paint, Role};

    /// An app variant's edit gets `ScrollAreaRecipes` and changes only what
    /// it names.
    #[test]
    fn edit_compiles_and_changes_its_part() {
        const LOUD: AppVariant<ScrollAreaVariant> = ScrollAreaVariant::app("loud");
        let t = Theme::vega().variant(
            LOUD,
            ScrollAreaVariant::Default,
            |r: &mut ScrollAreaRecipes, _| r.scroll_area_thumb.recolor(Role::Border, Role::Primary),
        );
        let v = t.resolve(ScrollAreaVariant::from(LOUD));
        let thumb = v.part(
            &t.style,
            |_, s| &s.scroll_area_thumb,
            |r| &r.scroll_area_thumb,
        );
        assert_eq!(
            thumb.bg.get(State::NONE),
            Some(Paint::Role(Role::Primary, 1.0))
        );
        let bar = v.part(
            &t.style,
            |_, s| &s.scroll_area_scrollbar,
            |r| &r.scroll_area_scrollbar,
        );
        assert_eq!(bar, &*t.style.scroll_area_scrollbar);
    }
}
