//! The [shadcn/ui](https://ui.shadcn.com/) design system for egui: the
//! shadcn v4 styles, colour themes, fonts, icon sets and presets, and
//! components drawn as plain egui widgets. Works with any egui backend.
//!
//! Apply a [`Theme`] (or a [`Preset`] from a shadcn preset code) once when
//! the context is created, then use the [`components`] like egui widgets:
//!
//! ```
//! use shadcn_egui::components::button::Button;
//! use shadcn_egui::Theme;
//!
//! fn setup(ctx: &egui::Context) {
//!     Theme::vega().apply(ctx);
//! }
//!
//! fn ui(ui: &mut egui::Ui) {
//!     if ui.add(Button::new("Save changes")).clicked() {
//!         // ...
//!     }
//! }
//! # let ctx = egui::Context::default();
//! # setup(&ctx);
//! # ctx.run_ui(Default::default(), |u| ui(u)).drop_without_applying_deltas();
//! ```
//!
//! Fonts other than Inter and icon sets other than Lucide are Cargo
//! features; the README lists them. The `gallery` example in the git
//! repository (not in the crates.io package) shows every component under
//! any preset: `cargo run --example gallery --features fonts-all,icons-all`.

mod box_shadow;
pub mod color;
pub mod components;
pub mod fonts;
pub mod icons;
pub mod layout;
mod palette;
pub mod preset;
pub mod reference;
pub mod style;
pub mod theme;
#[cfg(test)]
mod viewport_cache_tests;

pub use layout::Orientation;
pub use preset::{
    BaseColor, FontHeading, FontName, IconLibrary, MenuAccent, MenuColor, Preset, PresetCodeError,
    RadiusChoice, StyleName, ThemeColor,
};
pub use shadcn_egui_core::*;
pub use style::Style;
pub use theme::{theme, Metrics, Palette, ShadowLayer, Theme, ThemeFonts};

// API decision D-1: a leaf widget has no `enabled` flag of its own; the
// `Ui` disables it (`ui.add_enabled(false, ..)`, `Ui::disable`), as an egui
// leaf. Stable rustdoc does not check the error code of a compile_fail
// block, so the first block builds the same eight leaves without
// `.enabled`: a constructor change breaks it instead of letting the
// others fail for another reason.
#[cfg(doctest)]
/// ```
/// use shadcn_egui::components::{button::Button, checkbox::Checkbox, input::Input};
/// use shadcn_egui::components::{label::Label, select::Select, slider::Slider};
/// use shadcn_egui::components::{switch::Switch, tabs::Tabs};
/// let _ = Button::new("Save");
/// let _ = Checkbox::new(&mut false);
/// let _ = Input::new(&mut String::new());
/// let _ = Label::new("Email");
/// let _ = Select::new("s", &mut 0, &["A"]);
/// let _ = Slider::new(&mut 0.0, 0.0..=1.0);
/// let _ = Switch::new(&mut false);
/// let _ = Tabs::new(&mut 0, &["A"]);
/// ```
///
/// ```compile_fail,E0599
/// use shadcn_egui::components::button::Button;
/// let _ = Button::new("Save").enabled(false);
/// ```
///
/// ```compile_fail,E0599
/// use shadcn_egui::components::checkbox::Checkbox;
/// let _ = Checkbox::new(&mut false).enabled(false);
/// ```
///
/// ```compile_fail,E0599
/// use shadcn_egui::components::input::Input;
/// let _ = Input::new(&mut String::new()).enabled(false);
/// ```
///
/// ```compile_fail,E0599
/// use shadcn_egui::components::label::Label;
/// let _ = Label::new("Email").enabled(false);
/// ```
///
/// ```compile_fail,E0599
/// use shadcn_egui::components::select::Select;
/// let _ = Select::new("s", &mut 0, &["A"]).enabled(false);
/// ```
///
/// ```compile_fail,E0599
/// use shadcn_egui::components::slider::Slider;
/// let _ = Slider::new(&mut 0.0, 0.0..=1.0).enabled(false);
/// ```
///
/// ```compile_fail,E0599
/// use shadcn_egui::components::switch::Switch;
/// let _ = Switch::new(&mut false).enabled(false);
/// ```
///
/// ```compile_fail,E0599
/// use shadcn_egui::components::tabs::Tabs;
/// let _ = Tabs::new(&mut 0, &["A"]).enabled(false);
/// ```
struct LeavesHaveNoEnabledFlag;

// The Claude Code skills under plugin/skills/ name this crate's API. Compiling
// their rust fences as doctests makes an API change fail `cargo test` until
// the skill text follows. cfg(doctest) drops these items before include_str!
// expands, so builds and `cargo package` never read the files, which lie
// outside the package.
#[cfg(doctest)]
mod skill_docs {
    #[doc = include_str!("../../../plugin/skills/build-ui/SKILL.md")]
    struct BuildUi;
    #[doc = include_str!("../../../plugin/skills/build-ui/references/component-map.md")]
    struct ComponentMap;
    #[doc = include_str!("../../../plugin/skills/build-ui/references/cpu-rendering.md")]
    struct CpuRendering;
    #[doc = include_str!("../../../plugin/skills/build-ui/references/gotchas.md")]
    struct Gotchas;
    #[doc = include_str!("../../../plugin/skills/build-ui/references/icons.md")]
    struct Icons;
    #[doc = include_str!("../../../plugin/skills/build-ui/references/layout-map.md")]
    struct LayoutMap;
    #[doc = include_str!("../../../plugin/skills/build-ui/references/presets.md")]
    struct Presets;
    #[doc = include_str!("../../../plugin/skills/build-ui/references/token-map.md")]
    struct TokenMap;
    #[doc = include_str!("../../../plugin/skills/port-component/SKILL.md")]
    struct PortComponent;
    #[doc = include_str!("../../../plugin/skills/port-component/references/in-app.md")]
    struct InApp;
    #[doc = include_str!("../../../plugin/skills/port-component/references/styles.md")]
    struct Styles;
    #[doc = include_str!("../../../plugin/skills/port-component/references/upstream.md")]
    struct Upstream;
}
