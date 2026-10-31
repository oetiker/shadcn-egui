mod common;

use egui::vec2;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use shadcn_egui::components::button::{Button, ButtonSize};
use shadcn_egui::components::item::Item;
use shadcn_egui::Theme;

#[derive(Default)]
struct Clicks {
    item: usize,
    button: usize,
}

/// A clickable item with an action button, counting the clicks of both.
fn clickable_item() -> Harness<'static, Clicks> {
    let theme = Theme::vega();
    Harness::builder()
        .with_size(vec2(400.0, 120.0))
        .build_ui_state(
            move |ui, s: &mut Clicks| {
                theme.apply(ui.ctx());
                let r = Item::new()
                    .title("Row")
                    .description("A row with an action")
                    .sense(egui::Sense::click())
                    .show(ui, |ui| ui.add(Button::new("Open").size(ButtonSize::Sm)));
                s.item += usize::from(r.response.clicked());
                s.button += usize::from(r.inner.clicked());
            },
            Clicks::default(),
        )
}

/// The item senses with the previous pass's rect before its content, so an
/// action button lies above it: a click on the button is the button's.
#[test]
fn a_click_on_an_action_reaches_the_action_not_the_item() {
    let mut h = clickable_item();
    h.run();
    h.get_by_label("Open").click();
    h.run();
    assert_eq!((h.state().button, h.state().item), (1, 0));
}

/// A click on the item outside its actions is the item's.
#[test]
fn a_click_beside_the_actions_reaches_the_item() {
    let mut h = clickable_item();
    h.run();
    let at = h.get_by_label("Row").rect().center();
    for e in common::click(at) {
        h.event(e);
    }
    h.run();
    assert_eq!((h.state().button, h.state().item), (0, 1));
}

/// The rows of an item are as high as what they measured in the last pass,
/// never the infinite room below them: the item has a finite height and
/// stops redoing passes once it has measured itself.
#[test]
fn an_item_settles_at_a_finite_height() {
    let ctx = egui::Context::default();
    let theme = Theme::vega();
    let mut rects = Vec::new();
    for _ in 0..3 {
        let _ = common::pass(common::Loop::RunUi, &ctx, vec![], |ui| {
            theme.apply(ui.ctx());
            let r = Item::new()
                .icon(shadcn_egui::icons::lucide::INBOX)
                .title("Row")
                .description("A row with an action")
                .show(ui, |ui| ui.add(Button::new("Open").size(ButtonSize::Sm)));
            rects.push((r.response.rect, ui.ctx().will_discard()));
        });
    }
    let (last, discard) = *rects.last().unwrap();
    assert!(
        last.height().is_finite() && last.height() < 200.0,
        "{last:?}"
    );
    assert!(!discard, "the third frame still discards");
}

/// Three frames of `add` in a `width` wide window, without animations.
fn settle<T>(theme: &Theme, width: f32, mut add: impl FnMut(&mut egui::Ui) -> T) -> T {
    let ctx = egui::Context::default();
    theme.apply(&ctx);
    ctx.all_styles_mut(|s| s.animation_time = 0.0);
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            vec2(width, 800.0),
        )),
        ..Default::default()
    };
    let mut last = None;
    for _ in 0..3 {
        let _ = ctx.run_ui(input(), |ui| last = Some(add(ui)));
    }
    last.expect("a frame ran")
}

/// The item and its action button, `width` wide.
fn row_rects(width: f32, title: &str, action: &str) -> [egui::Rect; 2] {
    settle(&Theme::vega(), 900.0, |ui| {
        ui.set_max_width(width);
        let r = Item::new()
            .icon(shadcn_egui::icons::lucide::INBOX)
            .title(title)
            .show(ui, |ui| {
                ui.add(Button::new(action).size(ButtonSize::Sm)).rect
            });
        [r.response.rect, r.inner]
    })
}

/// With room, the actions end the line, centred on it.
#[test]
fn actions_end_the_line_when_it_has_room() {
    let [item, button] = row_rects(400.0, "Row", "Open");
    // vega: `border px-4`.
    assert_eq!(button.right(), item.right() - 17.0);
    assert!(
        (button.center().y - item.center().y).abs() < 0.5,
        "{button:?} in {item:?}"
    );
}

/// `flex-wrap`: once media, the content's longest word, the actions and
/// the gaps exceed the width, the actions take a line of their own below,
/// at the line's start.
#[test]
fn actions_wrap_below_a_narrow_line() {
    let [wide, _] = row_rects(400.0, "Notifications", "Mark all as read");
    let [item, button] = row_rects(200.0, "Notifications", "Mark all as read");
    assert_eq!(button.left(), item.left() + 17.0);
    assert!(
        item.height() > wide.height() + button.height(),
        "{item:?} vs {wide:?}"
    );
    assert!(button.bottom() < item.bottom());
}

/// The longest word counts, not the whole title: a title of short words
/// wraps beside the actions; one word as long does not fit beside them.
#[test]
fn the_longest_word_decides_the_wrap() {
    let words = "Aa bb cc dd ee ff gg hh ii jj kk";
    let one = "Aabbccddeeffgghhiijjkk";
    let [item, button] = row_rects(260.0, words, "Open");
    assert_eq!(button.right(), item.right() - 17.0, "short words wrap");
    let [item, button] = row_rects(260.0, one, "Open");
    assert_eq!(
        button.left(),
        item.left() + 17.0,
        "a long word wraps the actions"
    );
}

/// In unbounded room the item is as wide as its media, content and
/// actions, never infinite, and the actions follow the content.
#[test]
fn unbounded_room_takes_the_content_width() {
    let (item, button) = settle(&Theme::vega(), 900.0, |ui| {
        ui.allocate_ui(vec2(f32::INFINITY, 200.0), |ui| {
            let r = Item::new().title("Saved").show(ui, |ui| {
                ui.add(Button::new("Undo").size(ButtonSize::Sm)).rect
            });
            (r.response.rect, r.inner)
        })
        .inner
    });
    assert!(item.width().is_finite() && item.width() < 300.0, "{item:?}");
    assert_eq!(button.right(), item.right() - 17.0);
}
