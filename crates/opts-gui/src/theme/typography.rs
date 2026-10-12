//! The OPTS type scale: nine styles (the Phase 9 decision, `design-language.md` section 5), each
//! with a size, a line height, a tracking, a weight and optionally tabular figures.
//!
//! The style table is applied by `sync_typestyle_tokens`: **size, line height, letter spacing,
//! weight, family and figure style** all reach the text, not only the size as before. No size is
//! below 12 px at 100 % UI scale ([`MIN_TEXT_PX`]).
//!
//! The 15 Material names of the old scale still exist as deprecated aliases that resolve to one
//! of the nine styles ([`TypeScale::canonical`]), so existing screens keep compiling while they
//! migrate.

/// The smallest text size, in logical pixels before UI scale (accessibility floor, UI-P10).
pub const MIN_TEXT_PX: f32 = 12.0;

/// The type scale roles.
///
/// The first nine variants are the scale; the remaining ones are deprecated aliases of the old
/// 15-step Material scale (see [`TypeScale::canonical`] for where each one lands).
#[allow(deprecated)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeScale {
    /// 40 px, hero titles only.
    Display,
    /// 28 px, screen titles.
    Headline,
    /// 20 px, dialog and section titles.
    TitleLarge,
    /// 16 px medium, card and list titles.
    TitleMedium,
    /// 16 px, long text.
    BodyLarge,
    /// 14 px, standard UI text.
    BodyMedium,
    /// 12 px medium, labels, chips, overlines, hints.
    Label,
    /// 48 px tabular, the speed readout.
    NumericLarge,
    /// 32 px tabular, secondary instrument readouts.
    NumericMedium,

    // ---- deprecated aliases of the former 15 styles -------------------------------------
    /// Deprecated: use [`TypeScale::Display`].
    #[deprecated(note = "use TypeScale::Display")]
    DisplayLarge,
    /// Deprecated: use [`TypeScale::Display`].
    #[deprecated(note = "use TypeScale::Display")]
    DisplayMedium,
    /// Deprecated: use [`TypeScale::Display`].
    #[deprecated(note = "use TypeScale::Display")]
    DisplaySmall,
    /// Deprecated: use [`TypeScale::Headline`].
    #[deprecated(note = "use TypeScale::Headline")]
    HeadlineLarge,
    /// Deprecated: use [`TypeScale::Headline`].
    #[deprecated(note = "use TypeScale::Headline")]
    HeadlineMedium,
    /// Deprecated: use [`TypeScale::Headline`].
    #[deprecated(note = "use TypeScale::Headline")]
    HeadlineSmall,
    /// Deprecated: use [`TypeScale::BodyMedium`] (medium weight).
    #[deprecated(note = "use TypeScale::BodyMedium")]
    TitleSmall,
    /// Deprecated: use [`TypeScale::BodyMedium`] (medium weight).
    #[deprecated(note = "use TypeScale::BodyMedium")]
    LabelLarge,
    /// Deprecated: use [`TypeScale::Label`].
    #[deprecated(note = "use TypeScale::Label")]
    LabelMedium,
    /// Deprecated: use [`TypeScale::Label`].
    #[deprecated(note = "use TypeScale::Label")]
    LabelSmall,
    /// Deprecated: use [`TypeScale::Label`] (the old 12 px body-small).
    #[deprecated(note = "use TypeScale::Label")]
    BodySmall,
}

#[allow(deprecated)]
impl TypeScale {
    /// The nine styles of the scale.
    pub const fn all() -> &'static [TypeScale] {
        &[
            TypeScale::Display,
            TypeScale::Headline,
            TypeScale::TitleLarge,
            TypeScale::TitleMedium,
            TypeScale::BodyLarge,
            TypeScale::BodyMedium,
            TypeScale::Label,
            TypeScale::NumericLarge,
            TypeScale::NumericMedium,
        ]
    }

    /// The scale style a (possibly deprecated) role resolves to.
    pub const fn canonical(self) -> TypeScale {
        match self {
            Self::DisplayLarge | Self::DisplayMedium | Self::DisplaySmall => Self::Display,
            Self::HeadlineLarge | Self::HeadlineMedium | Self::HeadlineSmall => Self::Headline,
            Self::TitleSmall | Self::LabelLarge => Self::BodyMedium,
            Self::LabelMedium | Self::LabelSmall | Self::BodySmall => Self::Label,
            other => other,
        }
    }

    /// A weight the deprecated alias asks for on top of its canonical style (the old
    /// `TitleSmall` and `LabelLarge` were medium weight).
    const fn legacy_weight(self) -> Option<u16> {
        match self {
            Self::TitleSmall | Self::LabelLarge => Some(500),
            _ => None,
        }
    }

    const fn index(self) -> usize {
        match self.canonical() {
            Self::Display => 0,
            Self::Headline => 1,
            Self::TitleLarge => 2,
            Self::TitleMedium => 3,
            Self::BodyLarge => 4,
            Self::BodyMedium => 5,
            Self::Label => 6,
            Self::NumericLarge => 7,
            _ => 8,
        }
    }
}

/// Specification of one text style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeStyle {
    /// Nominal font size in logical pixels (before UI scale).
    pub font_size: f32,
    /// Line height in logical pixels (before UI scale).
    pub line_height: f32,
    /// Letter spacing in logical pixels (before UI scale).
    pub tracking: f32,
    /// CSS-style weight: 400 regular, 500 medium, 700 bold (the bundled faces; 600 would resolve
    /// to bold).
    pub weight: u16,
    /// Use tabular (fixed-width) figures so values do not jitter.
    pub tabular: bool,
}

impl TypeStyle {
    /// Creates a regular-weight, proportional-figure style.
    pub const fn new(font_size: f32, line_height: f32, tracking: f32) -> Self {
        Self {
            font_size,
            line_height,
            tracking,
            weight: 400,
            tabular: false,
        }
    }

    /// The same style with another weight.
    pub const fn weight(mut self, weight: u16) -> Self {
        self.weight = weight;
        self
    }

    /// The same style with tabular figures.
    pub const fn tabular(mut self) -> Self {
        self.tabular = true;
        self
    }

    /// Line height as a multiple of the font size (what the text system takes, so it scales).
    pub fn line_height_ratio(&self) -> f32 {
        self.line_height / self.font_size
    }
}

/// The table of the nine styles.
#[derive(Debug, Clone, PartialEq)]
pub struct TypographyTokens {
    styles: [TypeStyle; 9],
}

impl TypographyTokens {
    /// The style of a role (deprecated aliases give their canonical style).
    pub fn get(&self, scale: TypeScale) -> &TypeStyle {
        &self.styles[scale.index()]
    }

    /// Mutable access to the style of a role (aliases edit their canonical style).
    pub fn get_mut(&mut self, scale: TypeScale) -> &mut TypeStyle {
        &mut self.styles[scale.index()]
    }

    /// The style a role is drawn with: [`get`](Self::get) plus the weight a deprecated alias asks
    /// for.
    pub fn resolve(&self, scale: TypeScale) -> TypeStyle {
        let mut style = *self.get(scale);
        if let Some(w) = scale.legacy_weight() {
            style.weight = w;
        }
        style
    }
}

impl Default for TypographyTokens {
    fn default() -> Self {
        Self {
            styles: [
                TypeStyle::new(40.0, 48.0, -0.5).weight(700),  // Display
                TypeStyle::new(28.0, 36.0, -0.25).weight(700), // Headline
                TypeStyle::new(20.0, 28.0, 0.0).weight(700),   // TitleLarge
                TypeStyle::new(16.0, 24.0, 0.1).weight(500),   // TitleMedium
                TypeStyle::new(16.0, 24.0, 0.1),               // BodyLarge
                TypeStyle::new(14.0, 20.0, 0.15),              // BodyMedium
                TypeStyle::new(MIN_TEXT_PX, 16.0, 0.4).weight(500), // Label
                TypeStyle::new(48.0, 56.0, 0.0).weight(700).tabular(), // NumericLarge
                TypeStyle::new(32.0, 40.0, 0.0).weight(700).tabular(), // NumericMedium
            ],
        }
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;

    #[test]
    fn the_scale_has_nine_styles_none_below_the_floor() {
        let typography = TypographyTokens::default();
        assert_eq!(TypeScale::all().len(), 9);
        for &scale in TypeScale::all() {
            let style = typography.get(scale);
            assert!(style.font_size >= MIN_TEXT_PX, "{scale:?}");
            assert!(style.line_height >= style.font_size, "{scale:?}");
        }
    }

    #[test]
    fn the_scale_matches_the_design_language() {
        let t = TypographyTokens::default();
        let sizes: Vec<f32> = TypeScale::all()
            .iter()
            .map(|&s| t.get(s).font_size)
            .collect();
        assert_eq!(
            sizes,
            [40.0, 28.0, 20.0, 16.0, 16.0, 14.0, 12.0, 48.0, 32.0]
        );
        assert!(t.get(TypeScale::NumericLarge).tabular);
        assert!(!t.get(TypeScale::BodyMedium).tabular);
    }

    #[test]
    fn deprecated_aliases_land_on_a_scale_style_and_never_below_the_floor() {
        let t = TypographyTokens::default();
        for old in [
            TypeScale::DisplayLarge,
            TypeScale::DisplayMedium,
            TypeScale::DisplaySmall,
            TypeScale::HeadlineLarge,
            TypeScale::HeadlineMedium,
            TypeScale::HeadlineSmall,
            TypeScale::TitleSmall,
            TypeScale::LabelLarge,
            TypeScale::LabelMedium,
            TypeScale::LabelSmall,
            TypeScale::BodySmall,
        ] {
            assert!(TypeScale::all().contains(&old.canonical()), "{old:?}");
            assert!(t.resolve(old).font_size >= MIN_TEXT_PX, "{old:?}");
        }
        assert_eq!(t.resolve(TypeScale::LabelLarge).weight, 500);
        assert_eq!(t.resolve(TypeScale::BodyMedium).weight, 400);
    }
}
