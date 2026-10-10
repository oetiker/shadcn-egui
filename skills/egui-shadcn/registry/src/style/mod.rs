//! shadcn styles: recipe types (hand-written, `recipe.rs`) and generated
//! style values (`generated.rs`, `<name>.rs`, by `tools/stylegen`).
//! See `references/styles.md` in the skill.

mod generated;
mod nova;
pub mod recipe;
mod vega;

pub use generated::Style;
pub use recipe::*;

impl Style {
    /// shadcn's classic look: new-york sizes on the current base.
    pub fn vega() -> Self {
        vega::style()
    }
    /// Reduced padding and margins for compact layouts.
    pub fn nova() -> Self {
        nova::style()
    }
}

impl Default for Style {
    fn default() -> Self {
        Style::vega()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_styles_carry_known_values() {
        let (v, n) = (Style::vega(), Style::nova());
        assert_eq!(v.name, "vega");
        assert_eq!(n.name, "nova");
        assert_eq!(v.button_size_default.height.get(State::NONE), Some(36.0));
        assert_eq!(n.button_size_default.height.get(State::NONE), Some(32.0));
        assert_eq!(n.button_size_default.pad_left.get(State::NONE), Some(10.0));
        assert_eq!(n.button_size_default.pad_left.get(State::ICON_START), Some(8.0));
        assert_eq!(n.card_content.pad_left.get(State::SIZE_SM), Some(12.0));
        assert_eq!(Style::default(), Style::vega());
    }
}
