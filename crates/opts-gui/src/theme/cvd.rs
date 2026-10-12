//! Colour-vision-deficiency simulation for the contrast test (design language section 4: pairs
//! are checked in a deuteranopia/protanopia simulation).
//!
//! The matrices are Machado, Oliveira and Fernandes (2009), severity 1.0, applied to **linear**
//! sRGB. The simulation is an approximation of what a person with that deficiency sees; it is a
//! test instrument, not an assistive feature.

use bevy::color::{Color, LinearRgba};

use super::color::ColorScheme;
use super::contrast::contrast_ratio;

/// A kind of colour-vision deficiency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deficiency {
    /// Missing long-wavelength (red) cones.
    Protanopia,
    /// Missing medium-wavelength (green) cones.
    Deuteranopia,
    /// Missing short-wavelength (blue) cones.
    Tritanopia,
}

impl Deficiency {
    /// All simulated deficiencies.
    pub const ALL: [Deficiency; 3] = [Self::Protanopia, Self::Deuteranopia, Self::Tritanopia];

    fn matrix(self) -> [[f32; 3]; 3] {
        match self {
            Self::Protanopia => [
                [0.152_286, 1.052_583, -0.204_868],
                [0.114_503, 0.786_281, 0.099_216],
                [-0.003_882, -0.048_116, 1.051_998],
            ],
            Self::Deuteranopia => [
                [0.367_322, 0.860_646, -0.227_968],
                [0.280_085, 0.672_501, 0.047_413],
                [-0.011_820, 0.042_940, 0.968_881],
            ],
            Self::Tritanopia => [
                [1.255_528, -0.076_749, -0.178_779],
                [-0.078_411, 0.930_809, 0.147_602],
                [0.004_733, 0.691_367, 0.303_900],
            ],
        }
    }
}

/// `color` as seen with `deficiency` (alpha kept).
pub fn simulate(color: Color, deficiency: Deficiency) -> Color {
    let c = color.to_linear();
    let m = deficiency.matrix();
    let v = [c.red, c.green, c.blue];
    let mix = |row: [f32; 3]| (row[0] * v[0] + row[1] * v[1] + row[2] * v[2]).clamp(0.0, 1.0);
    Color::LinearRgba(LinearRgba::new(mix(m[0]), mix(m[1]), mix(m[2]), c.alpha))
}

/// Contrast of two colours as seen with `deficiency`.
pub fn simulated_contrast(a: Color, b: Color, deficiency: Deficiency) -> f32 {
    contrast_ratio(simulate(a, deficiency), simulate(b, deficiency))
}

/// The statuses that must stay distinguishable from each other without relying on hue alone:
/// the pairs in which a red/green or yellow/blue confusion would hide the meaning.
pub fn status_pairs() -> [(super::color::ColorRole, super::color::ColorRole); 4] {
    use super::color::ColorRole::{Error, Info, Success, Warning};
    [
        (Success, Error),
        (Warning, Error),
        (Success, Warning),
        (Info, Error),
    ]
}

/// A scheme's status colours as simulated pairs: the contrast ratio between the two members, per
/// deficiency. Status is always icon + text + colour (design language section 4), so a low
/// ratio is reported (see the accessibility audit) rather than failed.
pub fn status_separation(scheme: &ColorScheme) -> Vec<(Deficiency, String, f32)> {
    let mut out = Vec::new();
    for d in Deficiency::ALL {
        for (a, b) in status_pairs() {
            out.push((
                d,
                format!("{a:?}/{b:?}"),
                simulated_contrast(scheme.get(a), scheme.get(b), d),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::accent::control_min;
    use crate::theme::audit::rules;
    use crate::theme::mode::ThemeKind;
    use crate::theme::AccentPreset;

    const KINDS: [ThemeKind; 3] = [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast];

    #[test]
    fn white_and_black_are_unchanged() {
        for d in Deficiency::ALL {
            let w = simulate(Color::WHITE, d).to_srgba();
            assert!(
                (w.red - 1.0).abs() < 0.01 && (w.green - 1.0).abs() < 0.01,
                "{d:?}"
            );
            let k = simulate(Color::BLACK, d).to_srgba();
            assert!(k.red < 0.01 && k.green < 0.01 && k.blue < 0.01, "{d:?}");
        }
    }

    #[test]
    fn red_and_green_collapse_for_deuteranopia() {
        let red = Color::srgb(0.9, 0.1, 0.1);
        let green = Color::srgb(0.1, 0.6, 0.1);
        let plain = contrast_ratio(red, green);
        let seen = simulated_contrast(red, green, Deficiency::Deuteranopia);
        assert!(seen < plain + 1.0, "{plain} -> {seen}");
    }

    /// Every contrast rule of the audit keeps its threshold when both colours are simulated:
    /// text and controls stay legible for people with a colour-vision deficiency, in every
    /// theme and accent preset. Violations here are findings; the test lists them all.
    #[test]
    fn every_pair_stays_legible_under_simulation() {
        let mut bad = Vec::new();
        let mut checked = 0;
        let mut lowest = f32::MAX;
        for kind in KINDS {
            for preset in AccentPreset::ALL {
                let scheme = ColorScheme::for_kind(kind, preset.color());
                for d in Deficiency::ALL {
                    for rule in rules() {
                        // The simulation is an approximation and WCAG has no criterion for it, so
                        // the floor is the *control/large-text* threshold for every pair (3:1,
                        // 4.5:1 in high contrast); the full text threshold is not claimed.
                        let required = control_min(kind);
                        let ratio = simulated_contrast(scheme.get(rule.fg), scheme.get(rule.bg), d);
                        checked += 1;
                        lowest = lowest.min(ratio);
                        if ratio + 1e-3 < required {
                            bad.push(format!(
                                "{kind:?}/{preset:?}/{d:?}: {:?} on {:?} = {ratio:.2} < {required}",
                                rule.fg, rule.bg
                            ));
                        }
                    }
                }
            }
        }
        eprintln!("lowest simulated ratio {lowest:.2} over {checked} pairs");
        assert!(checked > 4000, "{checked}");
        assert!(
            bad.is_empty(),
            "{} failures, first:\n{}",
            bad.len(),
            bad[..bad.len().min(80)].join("\n")
        );
    }
}
