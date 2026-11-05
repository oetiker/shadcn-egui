mod common;

use egui::load::{ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint};
use egui::{vec2, Color32, ColorImage, Context};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use shadcn_egui::components::avatar::{
    avatar_group_count, Avatar, AvatarBadge, AvatarGroup, AvatarGroupCount, AvatarRecipes,
    AvatarSize, AvatarVariant,
};
use shadcn_egui::style::{AppVariant, Paint, Role};
use shadcn_egui::{layout, Theme};
use std::sync::Arc;

const FACE: &str = "fixture://face";
const PENDING: &str = "fixture://pending";

/// A loader of two fixture images, so no test needs a network or a decoder:
/// `fixture://face`, a light sky over a dark head, and `fixture://pending`,
/// which never finishes loading.
struct Fixtures;

impl ImageLoader for Fixtures {
    fn id(&self) -> &str {
        "avatar-fixtures"
    }
    fn load(&self, _: &Context, uri: &str, _: SizeHint) -> ImageLoadResult {
        match uri {
            FACE => {
                let (w, h) = (96usize, 96usize);
                let pixels = (0..w * h)
                    .map(|i| {
                        let (x, y) = ((i % w) as f32 - 48.0, (i / w) as f32 - 48.0);
                        if x * x + (y - 10.0) * (y - 10.0) < 26.0 * 26.0 {
                            Color32::from_rgb(120, 70, 40)
                        } else if y > 30.0 {
                            Color32::from_rgb(40, 60, 140)
                        } else {
                            Color32::from_rgb(235, 240, 250)
                        }
                    })
                    .collect();
                Ok(ImagePoll::Ready {
                    image: Arc::new(ColorImage::new([w, h], pixels)),
                })
            }
            PENDING => Ok(ImagePoll::Pending { size: None }),
            _ => Err(LoadError::NotSupported),
        }
    }
    fn forget(&self, _: &str) {}
    fn forget_all(&self) {}
    fn byte_size(&self) -> usize {
        0
    }
}

fn install(ctx: &Context) {
    let has = ctx
        .loaders()
        .image
        .lock()
        .iter()
        .any(|l| l.id() == "avatar-fixtures");
    if !has {
        ctx.add_image_loader(Arc::new(Fixtures));
    }
}

/// A harness around `add` under vega light, the loader installed.
fn harness(add: impl FnMut(&mut egui::Ui) + 'static) -> Harness<'static> {
    let mut add = add;
    let mut h = Harness::builder()
        .with_size(vec2(320.0, 120.0))
        .build_ui(move |ui| {
            Theme::vega().apply(ui.ctx());
            install(ui.ctx());
            add(ui);
        });
    h.run();
    h
}

const ONLINE: AppVariant<AvatarVariant> = AvatarVariant::app("online");
const GREEN: Role = Role::custom("green");

/// The demos' `bg-green-600 dark:bg-green-800` badge as an app variant.
fn with_online(theme: Theme) -> Theme {
    theme
        .color(
            GREEN,
            shadcn_egui::color::oklch_to_srgb(0.627, 0.194, 149.214),
            shadcn_egui::color::oklch_to_srgb(0.448, 0.119, 151.328),
        )
        .variant(
            ONLINE,
            AvatarVariant::Default,
            |r: &mut AvatarRecipes, _| r.avatar_badge.bg.set(Paint::Role(GREEN, 1.0)),
        )
}

/// `size-6`, `size-8`, `data-[size=lg]:size-10`.
#[test]
fn sizes_follow_the_style() {
    for (size, side) in [
        (AvatarSize::Sm, 24.0),
        (AvatarSize::Default, 32.0),
        (AvatarSize::Lg, 40.0),
    ] {
        let r = common::measure(&Theme::vega(), egui::Theme::Light, move |ui| {
            ui.add(Avatar::new("CN").size(size))
        });
        assert_eq!(r.size(), vec2(side, side), "{size:?}");
    }
}

/// Without an image, and while it loads or after it failed, the fallback
/// shows and names the avatar; a loaded image replaces it and names it by
/// its `alt`.
#[test]
fn the_image_replaces_the_fallback_once_loaded() {
    for (src, image) in [
        (None, false),
        (Some(""), false),
        (Some(PENDING), false),
        (Some("fixture://missing"), false),
        (Some(FACE), true),
    ] {
        let h = harness(move |ui| {
            let mut a = Avatar::new("CN").alt_text("@shadcn");
            if let Some(src) = src {
                a = a.image(src);
            }
            ui.add(a);
        });
        let label = if image { "@shadcn" } else { "CN" };
        let node = h.get_by_label(label);
        let role = if image {
            egui::accesskit::Role::Image
        } else {
            egui::accesskit::Role::Label
        };
        assert_eq!(node.accesskit_node().role(), role, "{src:?}");
    }
}

/// `-space-x-2`: neighbours overlap by 8 px.
#[test]
fn a_group_overlaps_its_avatars() {
    let h = harness(|ui| {
        AvatarGroup::new().show(ui, |ui| {
            ui.add(Avatar::new("AA"));
            ui.add(Avatar::new("BB"));
            avatar_group_count(ui, "+3");
        });
    });
    let (a, b, c) = (
        h.get_by_label("AA").rect(),
        h.get_by_label("BB").rect(),
        h.get_by_label("+3").rect(),
    );
    assert_eq!(b.left() - a.right(), -8.0);
    assert_eq!(c.left() - b.right(), -8.0);
    assert_eq!(c.size(), vec2(32.0, 32.0));
}

/// `group-has-data-[size=lg]/avatar-group:size-10`: the count follows a
/// large avatar in its group, in the first frame.
#[test]
fn the_count_follows_the_groups_avatar_size() {
    for (size, side) in [(AvatarSize::Lg, 40.0), (AvatarSize::Sm, 24.0)] {
        let h = harness(move |ui| {
            AvatarGroup::new().show(ui, |ui| {
                ui.add(Avatar::new("AA").size(size));
                ui.add(AvatarGroupCount::new("+3"));
            });
        });
        assert_eq!(h.get_by_label("+3").rect().size(), vec2(side, side));
    }
}

/// The avatar demos in every style, light and dark: sizes, fallback, image,
/// badges and groups. The demos' `grayscale` class is left out (no filter
/// in the recipes yet).
#[test]
fn avatar_snapshots() {
    let plus = shadcn_egui::icons::lucide::PLUS;
    common::snapshot_all_with(
        "avatar",
        vec2(420.0, 120.0),
        move |ui| {
            let t = with_online(Theme::current(ui.ctx()));
            t.apply(ui.ctx());
            install(ui.ctx());
            layout::vstack(ui, 16.0, |ui| {
                layout::row(ui, 8.0, |ui| {
                    for size in [AvatarSize::Sm, AvatarSize::Default, AvatarSize::Lg] {
                        ui.add(Avatar::new("CN").image(FACE).size(size));
                    }
                    for size in [AvatarSize::Sm, AvatarSize::Default, AvatarSize::Lg] {
                        ui.add(Avatar::new("CN").size(size));
                    }
                    ui.add(
                        Avatar::new("ER")
                            .image(FACE)
                            .variant(ONLINE)
                            .badge(AvatarBadge::new()),
                    );
                    ui.add(
                        Avatar::new("PP")
                            .size(AvatarSize::Lg)
                            .badge(AvatarBadge::new().icon(plus)),
                    );
                    ui.add(Avatar::icon_only(plus, "Add"));
                });
                layout::row(ui, 24.0, |ui| {
                    AvatarGroup::new().show(ui, |ui| {
                        ui.add(Avatar::new("CN").image(FACE));
                        ui.add(Avatar::new("LR"));
                        ui.add(Avatar::new("ER").image(FACE));
                        avatar_group_count(ui, "+3");
                    });
                    AvatarGroup::new().show(ui, |ui| {
                        for name in ["CN", "LR", "ER"] {
                            ui.add(Avatar::new(name).size(AvatarSize::Lg));
                        }
                        ui.add(AvatarGroupCount::icon_only(plus, "More"));
                    });
                });
            });
        },
        |_| {},
    );
}
