//! Vertical Tabs (`orientation="vertical"`): the list as a column, the
//! keys on the vertical axis, the line indicator on the right, the
//! orientation in the accessibility tree, the list beside its content,
//! and snapshots.

mod common;

use egui::accesskit::{Orientation as AxOrientation, Role};
use egui_kittest::kittest::{NodeT, Queryable};
use shadcn_egui::components::tabs::{Tabs, TabsVariant};
use shadcn_egui::{layout, Orientation, Theme};
use std::cell::Cell;
use std::rc::Rc;

const LABELS: [&str; 3] = ["Account", "Password", "Notifications"];

/// A harness with one tab list of `LABELS`, selected index in the cell.
fn harness(
    orientation: Orientation,
    variant: TabsVariant,
) -> (egui_kittest::Harness<'static>, Rc<Cell<usize>>) {
    let active = Rc::new(Cell::new(0));
    let seen = active.clone();
    let h = egui_kittest::Harness::new_ui(move |ui| {
        Theme::vega().apply(ui.ctx());
        let mut sel = seen.get();
        ui.add(
            Tabs::new(&mut sel, &LABELS)
                .orientation(orientation)
                .variant(variant),
        );
        seen.set(sel);
    });
    (h, active)
}

/// `group-data-vertical/tabs:flex-col`, `w-full`: the triggers stack in
/// a column, every one as wide as the widest, with the list's gap
/// between them, inside the list's padding.
#[test]
fn vertical_list_stacks_its_triggers_in_a_column() {
    for (name, theme, mode) in common::themes() {
        for (variant, gap) in [(TabsVariant::Default, 0.0), (TabsVariant::Line, 4.0)] {
            let list = Rc::new(Cell::new(egui::Rect::NOTHING));
            let sink = list.clone();
            let h =
                common::page_harness(egui::vec2(400.0, 300.0), theme.clone(), mode, move |ui| {
                    let resp = ui.add(
                        Tabs::new(&mut 0, &LABELS)
                            .orientation(Orientation::Vertical)
                            .variant(variant),
                    );
                    sink.set(resp.rect);
                });
            let list = list.get();
            let t: Vec<egui::Rect> = LABELS.iter().map(|l| h.get_by_label(l).rect()).collect();
            let widest = t.iter().map(|r| r.width()).fold(0.0, f32::max);
            for r in &t {
                assert_eq!(r.width(), widest, "{name} {variant:?}: {t:?}");
                assert_eq!(r.left(), t[0].left(), "{name} {variant:?}");
            }
            for w in t.windows(2) {
                assert_eq!(w[1].top() - w[0].bottom(), gap, "{name} {variant:?}: {t:?}");
            }
            assert!(
                list.contains_rect(t[0]) && list.contains_rect(t[2]),
                "{name}"
            );
            assert!(list.height() > list.width() / 2.0, "{name}: {list:?}");
            // The list is as wide as its widest trigger (`w-fit`).
            let pad = t[0].left() - list.left();
            assert!(
                (list.right() - t[0].right() - pad).abs() < 0.01,
                "{name} {variant:?}: {list:?} {t:?}"
            );
        }
    }
}

/// bits' roving focus on a vertical list: ArrowDown/ArrowUp move the
/// selection and wrap, Home/End jump, ArrowLeft/ArrowRight do nothing.
#[test]
fn vertical_list_moves_on_up_and_down_only() {
    let (mut h, active) = harness(Orientation::Vertical, TabsVariant::Default);
    h.run();
    h.get_by_label("Account").focus();
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(active.get(), 1, "ArrowDown selects the next tab");
    h.key_press(egui::Key::ArrowRight);
    h.run();
    assert_eq!(active.get(), 1, "ArrowRight does nothing");
    h.key_press(egui::Key::ArrowLeft);
    h.run();
    assert_eq!(active.get(), 1, "ArrowLeft does nothing");
    h.key_press(egui::Key::ArrowDown);
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(active.get(), 0, "ArrowDown wraps to the first tab");
    h.key_press(egui::Key::ArrowUp);
    h.run();
    assert_eq!(active.get(), 2, "ArrowUp wraps to the last tab");
    h.key_press(egui::Key::Home);
    h.run();
    assert_eq!(active.get(), 0, "Home");
    h.key_press(egui::Key::End);
    h.run();
    assert_eq!(active.get(), 2, "End");
    let focused = h
        .query_all(egui_kittest::kittest::by())
        .find(|n| n.accesskit_node().is_focused())
        .and_then(|n| n.accesskit_node().label());
    assert_eq!(focused.as_deref(), Some("Notifications"), "focus follows");
}

/// A horizontal list ignores the vertical arrows.
#[test]
fn horizontal_list_ignores_up_and_down() {
    let (mut h, active) = harness(Orientation::Horizontal, TabsVariant::Default);
    h.run();
    h.get_by_label("Account").focus();
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(active.get(), 0, "ArrowDown");
    h.key_press(egui::Key::ArrowUp);
    h.run();
    assert_eq!(active.get(), 0, "ArrowUp");
}

/// bits' roving focus: an arrow off the list's axis does nothing, and the
/// focus stays on the trigger, though a focusable widget lies in that
/// direction (beside a vertical list, below a horizontal one).
#[test]
fn off_axis_arrows_keep_the_focus_in_the_list() {
    for o in [Orientation::Horizontal, Orientation::Vertical] {
        let active = Rc::new(Cell::new(0));
        let seen = active.clone();
        let mut h = egui_kittest::Harness::new_ui(move |ui| {
            Theme::vega().apply(ui.ctx());
            let mut sel = seen.get();
            let tabs = Tabs::new(&mut sel, &LABELS).orientation(o);
            if o == Orientation::Vertical {
                layout::row_top(ui, 8.0, |ui| {
                    ui.add(tabs);
                    let _ = ui.button("Neighbour");
                });
            } else {
                layout::vstack(ui, 8.0, |ui| {
                    ui.add(tabs);
                    let _ = ui.button("Neighbour");
                });
            }
            seen.set(sel);
        });
        h.run();
        h.get_by_label("Account").focus();
        h.run();
        let keys = if o == Orientation::Vertical {
            [egui::Key::ArrowRight, egui::Key::ArrowLeft]
        } else {
            [egui::Key::ArrowDown, egui::Key::ArrowUp]
        };
        for key in keys {
            h.key_press(key);
            h.run();
            let focused = h
                .query_all(egui_kittest::kittest::by())
                .find(|n| n.accesskit_node().is_focused())
                .and_then(|n| n.accesskit_node().label());
            assert_eq!(focused.as_deref(), Some("Account"), "{o:?} {key:?}");
            assert_eq!(active.get(), 0, "{o:?} {key:?}");
        }
    }
}

/// bits' list: `role="tablist"` with `aria-orientation`, its node over
/// the whole list.
#[test]
fn list_reports_its_orientation() {
    for (o, want) in [
        (Orientation::Horizontal, AxOrientation::Horizontal),
        (Orientation::Vertical, AxOrientation::Vertical),
    ] {
        let (mut h, _) = harness(o, TabsVariant::Default);
        h.run();
        let list = h.get_by_role(Role::TabList);
        assert_eq!(list.accesskit_node().orientation(), Some(want), "{o:?}");
        // The node covers the list, so a screen reader can show it.
        let bounds = list.rect();
        assert!(bounds.area() > 0.0, "{o:?}: {bounds:?}");
        for tab in h.get_all_by_role(Role::Tab) {
            assert!(bounds.contains_rect(tab.rect()), "{o:?}: {bounds:?}");
        }
    }
}

/// bits' trigger: `role="tab"` with `aria-selected`, inside the
/// `tablist`, in both orientations; not a toggle (`aria-pressed`).
#[test]
fn triggers_are_tabs_with_a_selected_state() {
    for o in [Orientation::Horizontal, Orientation::Vertical] {
        let (mut h, active) = harness(o, TabsVariant::Default);
        active.set(1);
        h.run();
        let tabs: Vec<_> = h.get_all_by_role(Role::Tab).collect();
        assert_eq!(tabs.len(), LABELS.len(), "{o:?}");
        for (i, label) in LABELS.iter().enumerate() {
            let tab = h.get_by_role_and_label(Role::Tab, label);
            let node = tab.accesskit_node();
            assert_eq!(node.is_selected(), Some(i == 1), "{o:?} {label}");
            assert_eq!(node.toggled(), None, "{o:?} {label}");
            assert_eq!(
                node.parent().map(|p| p.role()),
                Some(Role::TabList),
                "{o:?} {label}"
            );
        }
    }
}

/// `group-data-vertical/tabs:after:inset-y-0 after:-right-1 after:w-0.5`:
/// the active trigger's bar runs its full padding-box height, 2 px wide,
/// its right edge 4 px past the padding box's right edge.
#[test]
fn vertical_line_bar_sits_on_the_right() {
    let mode = egui::Theme::Light;
    let theme = Theme::vega();
    let fg = theme.palette_of(mode).foreground;
    let (list, out) = common::measure_output(&theme, mode, |ui| {
        ui.ctx().enable_accesskit();
        ui.add(
            Tabs::new(&mut 1, &LABELS)
                .orientation(Orientation::Vertical)
                .variant(TabsVariant::Line),
        )
    });
    let trigger = common::bounds_by_label(&out, "Password").expect("trigger");
    let bars: Vec<_> = common::rect_shapes(&out)
        .into_iter()
        .filter(|r| r.fill == fg && r.rect.width() == 2.0)
        .collect();
    assert_eq!(bars.len(), 1, "{bars:?}");
    let bar = bars[0].rect;
    // vega's trigger border is 1 px.
    assert_eq!(bar.top(), trigger.top() + 1.0, "{bar:?} {trigger:?}");
    assert_eq!(bar.bottom(), trigger.bottom() - 1.0, "{bar:?} {trigger:?}");
    assert_eq!(
        bar.right(),
        trigger.right() - 1.0 + 4.0,
        "{bar:?} {trigger:?}"
    );
    assert!(bar.right() <= list.right() + 0.01, "{bar:?} {list:?}");
}

/// The root's `flex` row: content sits `Tabs::content_gap` right of the
/// list, both at the top of the row though the content is taller
/// (`h-fit` keeps the list out of the row's stretch).
#[test]
fn row_top_puts_the_content_beside_the_list_at_the_top() {
    let rects = Rc::new(Cell::new((egui::Rect::NOTHING, egui::Rect::NOTHING, 0.0)));
    let sink = rects.clone();
    let _h = common::page_harness(
        egui::vec2(500.0, 400.0),
        Theme::vega(),
        egui::Theme::Light,
        move |ui| {
            let gap = Tabs::content_gap(ui);
            layout::row_top(ui, gap, |ui| {
                let list = ui.add(Tabs::new(&mut 0, &LABELS).orientation(Orientation::Vertical));
                let (content, _) =
                    ui.allocate_exact_size(egui::vec2(200.0, 300.0), egui::Sense::hover());
                sink.set((list.rect, content, gap));
            });
        },
    );
    let (list, content, gap) = rects.get();
    assert_eq!(gap, 8.0, "vega `gap-2`");
    assert_eq!(content.left() - list.right(), gap, "{list:?} {content:?}");
    assert_eq!(list.top(), content.top(), "{list:?} {content:?}");
}

#[test]
fn tabs_vertical_snapshot() {
    common::snapshot_all("tabs_vertical", egui::vec2(460.0, 300.0), |ui| {
        let gap = Tabs::content_gap(ui);
        // The upstream demo (`create/tabs/tabs-vertical.svelte`): the list
        // beside a bordered content box, which stretches to the row's
        // height (the list is `h-fit`, the box is not).
        layout::row_top(ui, gap, |ui| {
            ui.add(Tabs::new(&mut 0, &LABELS).orientation(Orientation::Vertical));
            let t = Theme::current(ui.ctx());
            // `style-vega:rounded-lg style-vega:p-6`, … per style.
            let (radius, pad) = match t.style.name {
                "nova" => (t.radius_lg(), 16.0),
                "lyra" => (0.0, 16.0),
                "maia" | "luma" | "rhea" => (t.radius_xl(), 24.0),
                "mira" => (t.radius_md(), 16.0),
                _ => (t.radius_lg(), 24.0),
            };
            // Read in the row, before the frame: inside it, it is `None`.
            let height = layout::stretch_height(ui).unwrap_or(0.0);
            egui::Frame::new()
                .stroke(egui::Stroke::new(1.0_f32, t.palette().border))
                .corner_radius(radius)
                .inner_margin(pad)
                .show(ui, |ui| {
                    // The rest of the row, the text wrapped inside it
                    // (a row's label does not wrap on its own).
                    ui.set_width(ui.available_width());
                    ui.set_min_height((height - 2.0 * (pad + 1.0)).max(0.0));
                    ui.add(
                        egui::Label::new(
                            "Manage your account preferences and profile information.",
                        )
                        .wrap(),
                    );
                });
        });
        ui.add_space(24.0);
        layout::row_top(ui, 32.0, |ui| {
            // Keyboard focus ring on an inactive trigger.
            let resp = ui.add(
                Tabs::new(&mut 1, &LABELS)
                    .orientation(Orientation::Vertical)
                    .variant(TabsVariant::Line),
            );
            ui.memory_mut(|m| m.request_focus(resp.id.with(("tab", 2))));
            ui.add_enabled(
                false,
                Tabs::new(&mut 0, &["Disabled", "Tabs"]).orientation(Orientation::Vertical),
            );
        });
    });
}
