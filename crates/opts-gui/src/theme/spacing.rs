//! The 4 px spacing grid, the density modes and the layout classes.
//!
//! Density changes the size of controls and the gaps between them: *Comfortable* (the default,
//! controls 40 px), *Compact* (32 px) and *Dense* (24 px, the editor). Layout classes pick the
//! page structure by window width.

use bevy::ui::Val;

/// Semantic spacing steps snapped to a 4 px grid (`Compact` and `Dense` are the two small
/// off-grid steps used by the density modes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpacingScale {
    /// 0 px.
    None,
    /// 2 px: the hairline gap of dense tool panels.
    Dense,
    /// 4 px.
    XSmall,
    /// 6 px: the tight gap of compact lists.
    Compact,
    /// 8 px.
    Small,
    /// 12 px.
    Medium,
    /// 16 px.
    Standard,
    /// 24 px.
    Large,
    /// 32 px.
    XLarge,
    /// 48 px.
    Huge,
}

impl SpacingScale {
    /// The pixel value of this step at *Comfortable* density.
    pub const fn px(&self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Dense => 2.0,
            Self::XSmall => 4.0,
            Self::Compact => 6.0,
            Self::Small => 8.0,
            Self::Medium => 12.0,
            Self::Standard => 16.0,
            Self::Large => 24.0,
            Self::XLarge => 32.0,
            Self::Huge => 48.0,
        }
    }

    /// The pixel value in a density mode: Comfortable as is, Compact 3/4 and Dense 1/2 of it,
    /// rounded to whole pixels, never below 2 px for a non-zero step.
    pub fn px_in(&self, density: Density) -> f32 {
        let base = self.px();
        if base == 0.0 {
            return 0.0;
        }
        (base * density.spacing_factor()).round().max(2.0)
    }

    /// The step as a Bevy [`Val::Px`] at Comfortable density.
    pub const fn val(&self) -> Val {
        Val::Px(self.px())
    }

    /// The step as a [`Val::Px`] in a density mode.
    pub fn val_in(&self, density: Density) -> Val {
        Val::Px(self.px_in(density))
    }
}

/// Spacing tokens configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingTokens {
    /// The base unit of the grid in pixels.
    pub base_unit: f32,
}

impl Default for SpacingTokens {
    fn default() -> Self {
        Self { base_unit: 4.0 }
    }
}

/// Information density of the interface (setting `density`).
#[derive(bevy::prelude::Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Density {
    /// The default: controls 40 px.
    #[default]
    Comfortable,
    /// Controls 32 px.
    Compact,
    /// Controls 24 px (the editor).
    Dense,
}

impl Density {
    /// All modes in display order.
    pub const ALL: [Density; 3] = [Self::Comfortable, Self::Compact, Self::Dense];

    /// Height of a standard control in logical pixels.
    pub const fn control_height(self) -> f32 {
        match self {
            Self::Comfortable => 40.0,
            Self::Compact => 32.0,
            Self::Dense => 24.0,
        }
    }

    /// Factor applied to spacing steps.
    pub const fn spacing_factor(self) -> f32 {
        match self {
            Self::Comfortable => 1.0,
            Self::Compact => 0.75,
            Self::Dense => 0.5,
        }
    }

    /// Stable key of the `density` setting.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Comfortable => "comfortable", // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Compact => "compact", // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Dense => "dense",     // l10n-ok: stored identifier or diagnostic, not UI copy
        }
    }

    /// Display label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Comfortable => "Comfortable", // l10n-ok: UI label until the settings page uses message keys (M-0063)
            Self::Compact => "Compact", // l10n-ok: UI label until the settings page uses message keys (M-0063)
            Self::Dense => "Dense", // l10n-ok: UI label until the settings page uses message keys (M-0063)
        }
    }

    /// Parses [`Density::key`] (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|d| d.key().eq_ignore_ascii_case(s.trim()))
    }
}

/// Window-width class that selects the page structure (navigation rail with or without labels,
/// master-detail side by side or stacked).
#[derive(
    bevy::prelude::Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord,
)]
pub enum LayoutClass {
    /// Narrower than 900 logical px.
    Compact,
    /// 900 up to and including 1400 logical px.
    #[default]
    Medium,
    /// Wider than 1400 logical px.
    Expanded,
}

impl LayoutClass {
    /// Upper bound (exclusive) of [`LayoutClass::Compact`].
    pub const COMPACT_BELOW: f32 = 900.0;
    /// Upper bound (inclusive) of [`LayoutClass::Medium`].
    pub const MEDIUM_UP_TO: f32 = 1400.0;

    /// The class of a width in logical pixels.
    pub fn from_width(width: f32) -> Self {
        if width < Self::COMPACT_BELOW {
            Self::Compact
        } else if width <= Self::MEDIUM_UP_TO {
            Self::Medium
        } else {
            Self::Expanded
        }
    }

    /// Page margin of the class in logical pixels (32 px expanded, 16 px compact).
    pub const fn page_margin(self) -> f32 {
        match self {
            Self::Compact => 16.0,
            Self::Medium => 24.0,
            Self::Expanded => 32.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_class_boundaries() {
        assert_eq!(LayoutClass::from_width(899.9), LayoutClass::Compact);
        assert_eq!(LayoutClass::from_width(900.0), LayoutClass::Medium);
        assert_eq!(LayoutClass::from_width(1400.0), LayoutClass::Medium);
        assert_eq!(LayoutClass::from_width(1400.1), LayoutClass::Expanded);
        assert_eq!(LayoutClass::from_width(1024.0), LayoutClass::Medium);
    }

    #[test]
    fn density_scales_spacing_and_controls() {
        assert_eq!(SpacingScale::Standard.px_in(Density::Comfortable), 16.0);
        assert_eq!(SpacingScale::Standard.px_in(Density::Compact), 12.0);
        assert_eq!(SpacingScale::Standard.px_in(Density::Dense), 8.0);
        assert_eq!(SpacingScale::None.px_in(Density::Dense), 0.0);
        assert!(SpacingScale::Dense.px_in(Density::Dense) >= 2.0);
        assert_eq!(Density::Comfortable.control_height(), 40.0);
        assert_eq!(Density::Compact.control_height(), 32.0);
        assert_eq!(Density::Dense.control_height(), 24.0);
        for d in Density::ALL {
            assert_eq!(Density::parse(d.key()), Some(d));
        }
    }

    #[test]
    fn controls_never_fall_below_the_target_size_floor_in_dense_mode_only() {
        // 32 px is the minimum target of the milestone; Dense (24 px) is the documented
        // editor-only exception.
        assert!(Density::Comfortable.control_height() >= 32.0);
        assert!(Density::Compact.control_height() >= 32.0);
    }
}
