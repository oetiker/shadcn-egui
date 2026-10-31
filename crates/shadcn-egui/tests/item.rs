mod common;

use egui::accesskit::Role;
use egui::vec2;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::components::button::{Button, ButtonSize};
use shadcn_egui::components::icon::Glyph;
use shadcn_egui::components::item::{Item, ItemGroup, ItemSize, ItemVariant};
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
    let at = h
        .get_by_role_and_label(egui::accesskit::Role::Button, "Row")
        .rect()
        .center();
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

/// A light vega context with AccessKit on, driven a frame at a time at a
/// clock the test sets.
struct Frames {
    ctx: egui::Context,
    time: f64,
}

impl Frames {
    fn new() -> Self {
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        Self { ctx, time: 0.0 }
    }

    /// One frame at `self.time` with `events`; `add` draws the page.
    fn run(
        &mut self,
        events: Vec<egui::Event>,
        add: impl FnMut(&mut egui::Ui),
    ) -> egui::FullOutput {
        let mut add = add;
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(400.0, 400.0),
            )),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        self.ctx.run_ui(input, |ui| {
            Theme::vega().apply(ui.ctx());
            ui.ctx().set_theme(egui::Theme::Light);
            add(ui);
        })
    }
}

/// The AccessKit node labelled `label`.
fn node<'a>(
    out: &'a egui::FullOutput,
    label: &str,
) -> (egui::accesskit::NodeId, &'a egui::accesskit::Node) {
    let update = out
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("accesskit on");
    let (id, n) = update
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some(label))
        .unwrap_or_else(|| panic!("no node {label}"));
    (*id, n)
}

/// The 3px focus ring strokes a frame painted.
fn rings(out: &egui::FullOutput) -> usize {
    common::rect_shapes(out)
        .iter()
        .filter(|r| r.stroke.width == 3.0)
        .count()
}

/// A plain button, then an item that senses `sense` (with `.link(link)`),
/// returning the item's response.
fn focus_page(sense: egui::Sense, link: bool) -> impl FnMut(&mut egui::Ui) -> egui::Response {
    move |ui| {
        let _ = ui.add(Button::new("Before"));
        ui.add(Item::new().title("Row").sense(sense).link(link))
    }
}

/// C 4.1: a press focuses a sensed item without a ring; a Tab focus shows
/// one.
#[test]
fn a_press_focuses_without_a_ring_a_tab_with_one() {
    for (sense, link) in [(egui::Sense::click(), false), (egui::Sense::hover(), true)] {
        let mut f = Frames::new();
        let mut page = focus_page(sense, link);
        let mut item = None;
        let out = f.run(vec![], |ui| item = Some(page(ui)));
        let at = item.as_ref().unwrap().rect.center();
        let _ = rings(&out);
        f.run(common::pointer_down(at), |ui| item = Some(page(ui)));
        let out = f.run(common::pointer_up(at), |ui| item = Some(page(ui)));
        assert!(
            item.as_ref().unwrap().has_focus(),
            "link {link}: a press focuses"
        );
        assert_eq!(rings(&out), 0, "link {link}: no ring after a press");

        let mut f = Frames::new();
        f.run(vec![], |ui| item = Some(page(ui)));
        f.run(common::key(egui::Key::Tab), |ui| item = Some(page(ui)));
        f.run(common::key(egui::Key::Tab), |ui| item = Some(page(ui)));
        let out = f.run(vec![], |ui| item = Some(page(ui)));
        assert!(
            item.as_ref().unwrap().has_focus(),
            "link {link}: Tab focuses"
        );
        assert_eq!(rings(&out), 1, "link {link}: a ring after Tab");
    }
}

/// Clicks of `item` after a Tab onto it and `keys`.
fn clicks_after(sense: egui::Sense, link: bool, keys: &[egui::Key]) -> usize {
    let mut f = Frames::new();
    let mut page = focus_page(sense, link);
    let mut clicks = 0;
    let mut frame = |f: &mut Frames, events| {
        f.run(events, |ui| clicks += usize::from(page(ui).clicked()));
    };
    frame(&mut f, vec![]);
    frame(&mut f, common::key(egui::Key::Tab));
    frame(&mut f, common::key(egui::Key::Tab));
    for k in keys {
        frame(&mut f, common::key_down(*k));
        frame(&mut f, common::key_up(*k));
    }
    frame(&mut f, vec![]);
    clicks
}

/// O2: a sensed item is a `<button>`, Enter and Space click it; a link
/// item is an `<a>`, only Enter does.
#[test]
fn enter_and_space_click_a_sensed_item_enter_alone_a_link() {
    let click = egui::Sense::click();
    assert_eq!(clicks_after(click, false, &[egui::Key::Enter]), 1);
    assert_eq!(clicks_after(click, false, &[egui::Key::Space]), 1);
    assert_eq!(
        clicks_after(egui::Sense::hover(), true, &[egui::Key::Enter]),
        1
    );
    assert_eq!(
        clicks_after(egui::Sense::hover(), true, &[egui::Key::Space]),
        0
    );
}

/// `Sense::CLICK` without `FOCUSABLE` clicks by pointer but takes no Tab
/// focus, so no key clicks it.
#[test]
fn an_item_without_focusable_takes_no_tab_and_no_keys() {
    let only = egui::Sense::CLICK;
    assert_eq!(
        clicks_after(only, false, &[egui::Key::Enter, egui::Key::Space]),
        0
    );
    let mut f = Frames::new();
    let mut page = focus_page(only, false);
    let mut item = None;
    for e in [
        vec![],
        common::key(egui::Key::Tab),
        common::key(egui::Key::Tab),
        vec![],
    ] {
        f.run(e, |ui| item = Some(page(ui)));
    }
    assert!(!item.unwrap().has_focus());
}

/// O5: `.link` and `.sense` add up, in either order: a link item with a
/// hover sense still clicks.
#[test]
fn link_and_sense_add_up_in_any_order() {
    type Build = fn() -> Item<'static>;
    let orders: [Build; 2] = [
        || {
            Item::new()
                .title("Row")
                .link(true)
                .sense(egui::Sense::hover())
        },
        || {
            Item::new()
                .title("Row")
                .sense(egui::Sense::hover())
                .link(true)
        },
    ];
    for build in orders {
        let mut f = Frames::new();
        let mut item = None;
        f.run(vec![], |ui| item = Some(ui.add(build())));
        let at = item.as_ref().unwrap().rect.center();
        let mut clicked = false;
        f.run(common::click(at), |ui| clicked = ui.add(build()).clicked());
        assert!(clicked);
    }
}

/// `aria-selected`: `.selected` reaches AccessKit; without it the item
/// reports no selection state.
#[test]
fn selected_reaches_accesskit() {
    for (sel, want) in [
        (Some(true), Some(true)),
        (Some(false), Some(false)),
        (None, None),
    ] {
        let mut f = Frames::new();
        let out = f.run(vec![], |ui| {
            let mut item = Item::new().title("Row").sense(egui::Sense::click());
            if let Some(s) = sel {
                item = item.selected(s);
            }
            ui.add(item);
        });
        let (_, n) = node(&out, "Row");
        assert_eq!(n.is_selected(), want, "{sel:?}");
        assert_eq!(n.toggled(), None, "{sel:?}: not a toggle button");
    }
}

/// Roles: a plain item is egui's `Other`, a sensed one a button, a link
/// item a link; the description is the node's description.
#[test]
fn an_items_role_follows_its_sense() {
    let cases: [(fn() -> Item<'static>, Role); 3] = [
        (|| Item::new().title("Row"), Role::Unknown),
        (
            || Item::new().title("Row").sense(egui::Sense::click()),
            Role::Button,
        ),
        (|| Item::new().title("Row").link(true), Role::Link),
    ];
    for (build, role) in cases {
        let mut f = Frames::new();
        let out = f.run(vec![], |ui| {
            ui.add(build().description("Details"));
        });
        let (_, n) = node(&out, "Row");
        assert_eq!(n.role(), role);
        assert_eq!(n.description(), Some("Details"));
    }
}

/// D10: the items are children of the group's `List` node.
#[test]
fn items_are_children_of_the_groups_list() {
    let mut f = Frames::new();
    let out = f.run(vec![], |ui| {
        ItemGroup::new().show(ui, |ui| {
            ui.add(Item::new().title("One"));
            ui.add(Item::new().title("Two"));
        });
    });
    let update = out.platform_output.accesskit_update.as_ref().unwrap();
    let (_, list) = update
        .nodes
        .iter()
        .find(|(_, n)| n.role() == egui::accesskit::Role::List)
        .expect("a List node");
    let kids = list.children();
    for label in ["One", "Two"] {
        let (id, _) = node(&out, label);
        assert!(kids.contains(&id), "{label} not a child of the list");
    }
}

/// D-88 (c), `transition-colors duration-100`: a link item's hover fill
/// fades in over 100 ms.
#[test]
fn the_hover_fill_fades_over_100_ms() {
    let muted = common::resolved(&Theme::vega(), egui::Theme::Light)
        .palette_of(egui::Theme::Light)
        .muted;
    let mut f = Frames::new();
    let mut item = None;
    let page = |ui: &mut egui::Ui| ui.add(Item::new().title("Row").link(true));
    f.run(vec![], |ui| item = Some(page(ui)));
    let rect = item.unwrap().rect;
    let fill = |out: &egui::FullOutput| {
        common::box_fills(out)
            .into_iter()
            .find(|(r, _)| r.contains(rect.center()) && r.width() >= rect.width() - 2.0)
            .map_or(egui::Color32::TRANSPARENT, |(_, c)| c)
    };
    f.time = 1.0;
    let start = f.run(vec![egui::Event::PointerMoved(rect.center())], |ui| {
        let _ = page(ui);
    });
    f.time = 1.05;
    let mid = f.run(vec![], |ui| {
        let _ = page(ui);
    });
    f.time = 1.2;
    let end = f.run(vec![], |ui| {
        let _ = page(ui);
    });
    assert_ne!(fill(&start), muted, "the fill jumped");
    let m = fill(&mid);
    assert!(m != muted && m.a() > 0, "half way: {m:?}");
    assert_eq!(fill(&end), muted);
}

/// O3: a word too long for the column is clipped at the content's edge.
#[test]
fn a_long_unbreakable_title_is_clipped() {
    let mut f = Frames::new();
    let mut item = None;
    let page = |ui: &mut egui::Ui| {
        ui.set_max_width(160.0);
        ui.add(Item::new().title("Supercalifragilisticexpialidocious-and-more"))
    };
    for _ in 0..2 {
        f.run(vec![], |ui| item = Some(page(ui)));
    }
    let out = f.run(vec![], |ui| item = Some(page(ui)));
    let rect = item.unwrap().rect;
    let clips: Vec<_> = out
        .shapes
        .iter()
        .filter(|c| matches!(c.shape, egui::Shape::Text(_)))
        .map(|c| c.clip_rect)
        .collect();
    assert!(!clips.is_empty());
    for clip in clips {
        // vega: `border px-4`.
        assert!(
            clip.right() <= rect.right() - 17.0 + 0.01,
            "{clip:?} in {rect:?}"
        );
    }
}

/// The gaps between the items of a group.
fn group_gaps(sizes: &[ItemSize]) -> Vec<f32> {
    let sizes = sizes.to_vec();
    let rects = settle(&Theme::vega(), 400.0, move |ui| {
        ItemGroup::new()
            .show(ui, |ui| {
                sizes
                    .iter()
                    .enumerate()
                    .map(|(i, z)| ui.add(Item::new().title(format!("Item {i}")).size(*z)).rect)
                    .collect::<Vec<_>>()
            })
            .inner
    });
    rects
        .windows(2)
        .map(|w| w[1].top() - w[0].bottom())
        .collect()
}

/// `gap-4 has-data-[size=sm]:gap-2.5 has-data-[size=xs]:gap-2`: the group
/// gap follows its items' sizes; with sm and xs items both rules match and
/// the later one in CSS order, xs, wins (D10).
#[test]
fn the_group_gap_follows_the_item_sizes() {
    use ItemSize::{Default, Sm, Xs};
    assert_eq!(group_gaps(&[Default, Default]), [16.0]);
    assert_eq!(group_gaps(&[Default, Sm]), [10.0]);
    assert_eq!(group_gaps(&[Xs, Default]), [8.0]);
    assert_eq!(group_gaps(&[Sm, Xs, Default]), [8.0, 8.0]);
}

/// The textured meshes a frame painted: the icons.
fn icon_meshes(out: &egui::FullOutput) -> Vec<egui::Mesh> {
    common::mesh_shapes(out)
        .into_iter()
        .filter(|m| m.texture_id != egui::TextureId::default())
        .collect()
}

/// `spinner-item-demo`: a `<Spinner />` in `Item.Media variant="icon"`
/// keeps its own `size-4` (the media's size is `:not([class*='size-'])`)
/// and takes the colour an icon there takes.
#[test]
fn a_spinner_media_keeps_its_size_and_the_icons_colour() {
    for (name, theme, mode) in common::themes() {
        let painted = |glyph: Glyph| {
            let (_, out) = common::measure_output(&theme, mode, |ui| {
                Item::new()
                    .variant(ItemVariant::Outline)
                    .icon(glyph)
                    .title("Downloading...")
                    .description("129 MB / 1000 MB")
                    .show(ui, |_| ())
                    .response
            });
            icon_meshes(&out)
        };
        let spinner = painted(Glyph::Spinner);
        let icon = painted(Glyph::Icon(shadcn_egui::icons::lucide::INBOX));
        assert_eq!(spinner.len(), 1, "{name}");
        assert_eq!(
            spinner[0].calc_bounds().size(),
            vec2(16.0, 16.0),
            "{name}"
        );
        assert_eq!(
            common::mesh_colour(&spinner[0]),
            common::mesh_colour(&icon[0]),
            "{name}"
        );
    }
}

/// The spinner stays a `role="status"` node named "Loading" inside the
/// item, and the item keeps its title as its name.
#[test]
fn a_spinner_media_is_a_status_inside_the_item() {
    let mut h = Harness::new_ui(|ui| {
        Item::new()
            .icon(Glyph::Spinner)
            .title("Downloading...")
            .sense(egui::Sense::click())
            .show(ui, |_| ());
    });
    h.run();
    let item = h.get_by_role_and_label(Role::Button, "Downloading...");
    let status = item.get_by_role(Role::Status);
    assert_eq!(status.accesskit_node().label().as_deref(), Some("Loading"));
    assert_eq!(
        status.accesskit_node().parent().map(|p| p.id()),
        Some(item.accesskit_node().id())
    );
}

/// `justify-between`: a header's or footer's first widget starts the row,
/// its last ends it, as far from the item's right edge as the first is
/// from its left; a single widget stays at the start.
#[test]
fn header_and_footer_spread_their_widgets() {
    use shadcn_egui::components::item::{item_footer, item_header};
    for footer in [false, true] {
        let [item, first, last] = settle(&Theme::vega(), 400.0, |ui| {
            let row = |ui: &mut egui::Ui| [ui.label("Start").rect, ui.label("End").rect];
            let r = Item::new().variant(ItemVariant::Outline).show(ui, |ui| {
                if footer {
                    item_footer(ui, row).inner
                } else {
                    item_header(ui, row).inner
                }
            });
            [r.response.rect, r.inner[0], r.inner[1]]
        });
        let inset = first.left() - item.left();
        assert!(inset > 0.0, "footer {footer}");
        assert!(
            (item.right() - last.right() - inset).abs() < 0.01,
            "footer {footer}: {item:?} {first:?} {last:?}"
        );
        assert!((first.top() - last.top()).abs() < 0.01, "footer {footer}");
    }
    let [item, only] = settle(&Theme::vega(), 400.0, |ui| {
        let r = Item::new().show(ui, |ui| item_header(ui, |ui| ui.label("Only").rect).inner);
        [r.response.rect, r.inner]
    });
    assert!(only.right() < item.center().x, "{item:?} {only:?}");
}

/// vega's `[a]:hover:bg-muted`: only a link item takes the hover fill; a
/// sensed item that is no link stays clear under the pointer.
#[test]
fn only_a_link_takes_the_hover_fill() {
    let muted = common::resolved(&Theme::vega(), egui::Theme::Light)
        .palette_of(egui::Theme::Light)
        .muted;
    for link in [false, true] {
        let mut f = Frames::new();
        let mut item = None;
        let page = |ui: &mut egui::Ui| {
            ui.add(
                Item::new()
                    .title("Row")
                    .sense(egui::Sense::click())
                    .link(link),
            )
        };
        f.run(vec![], |ui| item = Some(page(ui)));
        let rect = item.unwrap().rect;
        f.time = 1.0;
        f.run(vec![egui::Event::PointerMoved(rect.center())], |ui| {
            let _ = page(ui);
        });
        f.time = 2.0;
        let out = f.run(vec![], |ui| {
            let _ = page(ui);
        });
        let filled = common::box_fills(&out)
            .into_iter()
            .any(|(r, c)| r.contains(rect.center()) && c == muted);
        assert_eq!(filled, link, "link {link}");
    }
}

/// The rects of the text shapes a frame painted, in paint order.
fn text_rects(out: &egui::FullOutput) -> Vec<egui::Rect> {
    out.shapes
        .iter()
        .filter_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.visual_bounding_rect()),
            _ => None,
        })
        .collect()
}

/// vega's `group-has-data-[slot=item-description]/item:translate-y-0.5
/// self-start`: with a description the media starts 2 px below the
/// title's top; without one it is centred on the title.
#[test]
fn media_starts_below_the_title_with_a_description() {
    for description in [false, true] {
        let (_, out) = common::measure_output(&Theme::vega(), egui::Theme::Light, |ui| {
            let mut item = Item::new()
                .icon(shadcn_egui::icons::lucide::INBOX)
                .title("Title");
            if description {
                item = item.description("A description of two words");
            }
            item.show(ui, |_| ()).response
        });
        let icon = icon_meshes(&out)[0].calc_bounds();
        let title = text_rects(&out)[0];
        if description {
            // The title's line box: the text's own top lies inside it.
            let line_top = out
                .shapes
                .iter()
                .find_map(|c| match &c.shape {
                    egui::Shape::Text(t) => Some(t.pos.y),
                    _ => None,
                })
                .unwrap();
            assert_eq!(icon.top(), line_top + 2.0, "{icon:?} {title:?}");
        } else {
            assert!(
                (icon.center().y - title.center().y).abs() < 1.0,
                "{icon:?} {title:?}"
            );
        }
    }
}

/// `size-10`, `group-data-[size=sm]/item:size-8`, `xs: size-6`: image
/// media is 40, 32 or 24 px, so a title-only item grows by the image
/// height less the title's line.
#[test]
fn image_media_follows_the_item_size() {
    for (size, side) in [(ItemSize::Default, 40.0), (ItemSize::Sm, 32.0), (ItemSize::Xs, 24.0)] {
        let height = |image: bool| {
            settle(&Theme::vega(), 400.0, |ui| {
                let mut item = Item::new().title("Track").size(size);
                if image {
                    item = item.image("bytes://none.png");
                }
                item.show(ui, |_| ()).response.rect.height()
            })
        };
        let grown = height(true) - height(false);
        assert_eq!(
            grown + text_line(size),
            side,
            "{size:?}"
        );
    }
}

/// The height of a title-only item's title line.
fn text_line(size: ItemSize) -> f32 {
    let (_, out) = common::measure_output(&Theme::vega(), egui::Theme::Light, |ui| {
        Item::new().title("Track").size(size).show(ui, |_| ()).response
    });
    out.shapes
        .iter()
        .find_map(|c| match &c.shape {
            egui::Shape::Text(t) => Some(t.galley.rect.height()),
            _ => None,
        })
        .expect("the title painted")
}

#[test]
#[should_panic(expected = "an item takes one media setter")]
fn a_second_media_setter_panics() {
    let _ = Item::new()
        .icon(shadcn_egui::icons::lucide::INBOX)
        .image("bytes://none.png");
}

#[test]
#[should_panic(expected = "item_header comes before the item's actions")]
fn a_header_after_the_actions_panics() {
    settle(&Theme::vega(), 400.0, |ui| {
        Item::new().title("Row").show(ui, |ui| {
            ui.add(Button::new("Open"));
            shadcn_egui::components::item::item_header(ui, |ui| ui.label("Late"));
        });
    });
}

#[test]
#[should_panic(expected = "item_footer comes last in an item")]
fn a_part_after_the_footer_panics() {
    use shadcn_egui::components::item::{item_footer, item_header};
    settle(&Theme::vega(), 400.0, |ui| {
        Item::new().title("Row").show(ui, |ui| {
            item_footer(ui, |ui| ui.label("Foot"));
            item_header(ui, |ui| ui.label("Head"));
        });
    });
}

/// The shapes of a group of one item, as text.
fn group_shapes(group: ItemVariant, item: Option<ItemVariant>) -> String {
    let (_, out) = common::measure_output(&Theme::vega(), egui::Theme::Light, |ui| {
        ItemGroup::new()
            .variant(group)
            .show(ui, |ui| {
                let i = Item::new().title("Row");
                ui.add(match item {
                    Some(v) => i.variant(v),
                    None => i,
                })
            })
            .response
    });
    format!("{:?}", out.shapes)
}

/// The group's variant reaches an item that names none; an item's own
/// variant wins over it.
#[test]
fn the_group_variant_reaches_its_items() {
    use ItemVariant::{Default, Outline};
    let outline = group_shapes(Default, Some(Outline));
    assert_eq!(group_shapes(Outline, None), outline);
    assert_ne!(group_shapes(Default, None), outline);
    assert_eq!(group_shapes(Outline, Some(Default)), group_shapes(Default, None));
}

/// vega's `my-2` separator between two items of a group: the group's
/// gap on both sides of it, its margins and its 1 px line.
#[test]
fn a_separator_takes_its_margins_and_the_group_gap() {
    use shadcn_egui::components::item::item_separator;
    let rects = settle(&Theme::vega(), 400.0, |ui| {
        ItemGroup::new()
            .show(ui, |ui| {
                let a = ui.add(Item::new().title("One")).rect;
                let s = item_separator(ui).rect;
                let b = ui.add(Item::new().title("Two")).rect;
                [a, s, b]
            })
            .inner
    });
    let [a, s, b] = rects;
    assert_eq!(s.height(), 1.0, "{s:?}");
    assert_eq!(s.top() - a.bottom(), 16.0 + 8.0, "{a:?} {s:?}");
    assert_eq!(b.top() - s.bottom(), 8.0 + 16.0, "{s:?} {b:?}");
}

/// `fixture://cover`, a wide image (96 x 64: a sky band over a dark
/// ground, a light disc left of centre), so `object-cover` visibly cuts its
/// sides; no test needs a network or a decoder.
struct Cover;

const COVER: &str = "fixture://cover";

impl egui::load::ImageLoader for Cover {
    fn id(&self) -> &str {
        "item-fixtures"
    }
    fn load(
        &self,
        _: &egui::Context,
        uri: &str,
        _: egui::load::SizeHint,
    ) -> egui::load::ImageLoadResult {
        if uri != COVER {
            return Err(egui::load::LoadError::NotSupported);
        }
        let (w, h) = (96usize, 64usize);
        let pixels = (0..w * h)
            .map(|i| {
                let (x, y) = ((i % w) as f32 - 40.0, (i / w) as f32 - 28.0);
                if x * x + y * y < 14.0 * 14.0 {
                    egui::Color32::from_rgb(250, 220, 120)
                } else if y > 10.0 {
                    egui::Color32::from_rgb(40, 70, 50)
                } else {
                    egui::Color32::from_rgb(110, 160, 220)
                }
            })
            .collect();
        Ok(egui::load::ImagePoll::Ready {
            image: std::sync::Arc::new(egui::ColorImage::new([w, h], pixels)),
        })
    }
    fn forget(&self, _: &str) {}
    fn forget_all(&self) {}
    fn byte_size(&self) -> usize {
        0
    }
}

fn install_cover(ctx: &egui::Context) {
    let has = ctx
        .loaders()
        .image
        .lock()
        .iter()
        .any(|l| l.id() == "item-fixtures");
    if !has {
        ctx.add_image_loader(std::sync::Arc::new(Cover));
    }
}

/// `item-variants-demo` and `item-size-demo`: the three variants with an
/// outline action, then an outline item at each size with icon media and
/// a chevron action.
#[test]
fn item_snapshot() {
    use shadcn_egui::icons::lucide;
    common::snapshot_all("item", vec2(460.0, 560.0), |ui| {
        ui.ctx().all_styles_mut(|s| s.animation_time = 0.0);
        shadcn_egui::layout::vstack(ui, 16.0, |ui| {
            for (v, name) in [
                (ItemVariant::Default, "Default Variant"),
                (ItemVariant::Outline, "Outline Variant"),
                (ItemVariant::Muted, "Muted Variant"),
            ] {
                Item::new()
                    .variant(v)
                    .title(name)
                    .description("Standard styling with subtle background and borders.")
                    .show(ui, |ui| {
                        ui.add(
                            Button::new("Open")
                                .variant(shadcn_egui::components::button::ButtonVariant::Outline)
                                .size(ButtonSize::Sm),
                        )
                    });
            }
            for size in [ItemSize::Default, ItemSize::Sm, ItemSize::Xs] {
                Item::new()
                    .variant(ItemVariant::Outline)
                    .size(size)
                    .icon(lucide::BADGE_CHECK)
                    .title("Your profile has been verified.")
                    .show(ui, |ui| {
                        ui.add(shadcn_egui::components::icon::IconView::new(
                            &lucide::CHEVRON_RIGHT,
                        ))
                    });
            }
        });
    });
}

/// The media kinds: icon (`item-icon-demo`), a spinner
/// (`spinner-item-demo`, with its progress footer), a cropped image
/// (`item-image-demo`) and an avatar (`item-avatar-demo`).
#[test]
fn item_media_snapshot() {
    use shadcn_egui::components::avatar::Avatar;
    use shadcn_egui::components::item::item_footer;
    use shadcn_egui::components::progress::Progress;
    use shadcn_egui::icons::lucide;
    common::snapshot_all("item_media", vec2(460.0, 470.0), |ui| {
        ui.ctx().all_styles_mut(|s| s.animation_time = 0.0);
        install_cover(ui.ctx());
        shadcn_egui::layout::vstack(ui, 16.0, |ui| {
            Item::new()
                .variant(ItemVariant::Outline)
                .icon(lucide::SHIELD_ALERT)
                .title("Security Alert")
                .description("New login detected from unknown device.")
                .show(ui, |ui| {
                    ui.add(Button::new("Review").size(ButtonSize::Sm))
                });
            Item::new()
                .variant(ItemVariant::Outline)
                .icon(Glyph::Spinner)
                .title("Downloading...")
                .description("129 MB / 1000 MB")
                .show(ui, |ui| {
                    ui.add(
                        Button::new("Cancel")
                            .variant(shadcn_egui::components::button::ButtonVariant::Outline)
                            .size(ButtonSize::Sm),
                    );
                    item_footer(ui, |ui| ui.add(Progress::new(0.75)));
                });
            Item::new()
                .variant(ItemVariant::Outline)
                .image(COVER)
                .alt_text("Midnight City Lights")
                .title("Midnight City Lights")
                .description("Electric Nights")
                .link(true)
                .show(ui, |ui| ui.label("3:45"));
            Item::new()
                .variant(ItemVariant::Outline)
                .media(|ui| {
                    ui.add(Avatar::new("ER"));
                })
                .title("Evil Rabbit")
                .description("Last seen 5 months ago")
                .show(ui, |ui| {
                    ui.add(
                        Button::new("Invite")
                            .variant(shadcn_egui::components::button::ButtonVariant::Outline)
                            .size(ButtonSize::Sm)
                            .icon(lucide::PLUS),
                    )
                });
        });
    });
}

/// `item-group-demo` with separators between its items, then items with
/// a header row (`item-header-demo`, two widgets spread) and a footer.
#[test]
fn item_group_snapshot() {
    use shadcn_egui::components::avatar::Avatar;
    use shadcn_egui::components::item::{item_footer, item_header, item_separator};
    use shadcn_egui::icons::lucide;
    common::snapshot_all("item_group", vec2(420.0, 560.0), |ui| {
        ui.ctx().all_styles_mut(|s| s.animation_time = 0.0);
        shadcn_egui::layout::vstack(ui, 24.0, |ui| {
            ItemGroup::new().variant(ItemVariant::Outline).show(ui, |ui| {
                for (i, (name, mail)) in [
                    ("shadcn", "shadcn@vercel.com"),
                    ("maxleiter", "maxleiter@vercel.com"),
                    ("evilrabbit", "evilrabbit@vercel.com"),
                ]
                .into_iter()
                .enumerate()
                {
                    if i > 0 {
                        item_separator(ui);
                    }
                    Item::new()
                        .media(move |ui| {
                            ui.add(Avatar::new(&name[..1]));
                        })
                        .title(name)
                        .description(mail)
                        .show(ui, |ui| {
                            ui.add(
                                Button::new("Add")
                                    .variant(shadcn_egui::components::button::ButtonVariant::Ghost)
                                    .size(ButtonSize::Icon)
                                    .icon(lucide::PLUS),
                            )
                        });
                }
            });
            Item::new()
                .variant(ItemVariant::Outline)
                .title("v0-1.5-sm")
                .description("Everyday tasks and UI generation.")
                .show(ui, |ui| {
                    item_header(ui, |ui| {
                        ui.add(shadcn_egui::components::icon::IconView::new(&lucide::MUSIC));
                        ui.label("Model");
                    });
                    item_footer(ui, |ui| {
                        ui.label("Updated 2 days ago");
                        ui.label("v1.5");
                    });
                });
        });
    });
}

/// D8: a link item under the pointer (vega's `[a]:hover:bg-muted`) and a
/// sensed item focused by Tab, with its ring.
#[test]
fn item_states_snapshot() {
    use shadcn_egui::icons::lucide;
    common::snapshot_all_with(
        "item_states",
        vec2(420.0, 220.0),
        |ui| {
            ui.ctx().all_styles_mut(|s| s.animation_time = 0.0);
            shadcn_egui::layout::vstack(ui, 16.0, |ui| {
                ui.add(
                    Item::new()
                        .variant(ItemVariant::Outline)
                        .icon(lucide::BADGE_CHECK)
                        .title("Hovered link")
                        .description("Your profile has been verified.")
                        .link(true),
                );
                ui.add(
                    Item::new()
                        .variant(ItemVariant::Outline)
                        .title("Focused item")
                        .description("Reached with Tab.")
                        .sense(egui::Sense::click()),
                );
            });
        },
        |h| {
            h.key_press(egui::Key::Tab);
            h.run();
            h.key_press(egui::Key::Tab);
            h.run();
            h.get_by_role_and_label(Role::Link, "Hovered link").hover();
            h.run();
        },
    );
}
