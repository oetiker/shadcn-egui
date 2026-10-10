//! shadcn styles: recipe types (hand-written, `recipe.rs`) and generated
//! style values (`generated.rs`, `<name>.rs`, by `tools/stylegen`).
//! See `references/styles.md` in the skill.

mod generated;
mod luma;
mod lyra;
mod maia;
mod mira;
mod nova;
pub mod recipe;
mod rhea;
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
    /// The maia style.
    pub fn maia() -> Self {
        maia::style()
    }
    /// The lyra style.
    pub fn lyra() -> Self {
        lyra::style()
    }
    /// The mira style.
    pub fn mira() -> Self {
        mira::style()
    }
    /// The luma style.
    pub fn luma() -> Self {
        luma::style()
    }
    /// The rhea style.
    pub fn rhea() -> Self {
        rhea::style()
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

    type Maker = fn() -> Style;

    #[test]
    fn every_exported_style_resolves_a_button_height() {
        // One at a time: a `Style` is large, seven of them overflow a test thread's stack.
        let makers: [(&str, Maker); 7] = [
            ("vega", Style::vega),
            ("nova", Style::nova),
            ("maia", Style::maia),
            ("lyra", Style::lyra),
            ("mira", Style::mira),
            ("luma", Style::luma),
            ("rhea", Style::rhea),
        ];
        for (name, make) in makers {
            let s = make();
            assert_eq!(s.name, name);
            let h = s.button_size_default.height.get(State::NONE);
            assert!(h.is_some_and(|h| h > 0.0), "{name}: button height {h:?}");
        }
    }
}
