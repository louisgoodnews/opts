//! The contrast rules every [`ColorScheme`] must satisfy and the checker/auto-corrector built on
//! them (used by the unit tests over every theme and accent, and by theme files).
//!
//! Text pairs need 4.5:1 (7:1 in the high-contrast theme); control boundaries and icons need 3:1
//! (4.5:1 in high contrast). `OutlineVariant`, `Scrim` and `Shadow` are decorative and exempt.

use bevy::color::Color;

use super::accent::{control_min, text_min, SURFACES};
use super::color::{ColorRole, ColorScheme};
use super::contrast::{contrast_ratio, ensure_contrast};

/// What a rule protects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    /// Text and icons that carry meaning: the text threshold.
    Text,
    /// Control boundaries (outlines, focus ring colour): the control threshold.
    Control,
}

/// One foreground/background pair that must keep its contrast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContrastRule {
    /// The colour drawn.
    pub fg: ColorRole,
    /// The colour it is drawn on.
    pub bg: ColorRole,
    /// Which threshold applies.
    pub kind: RuleKind,
}

/// A rule that a concrete scheme breaks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Violation {
    /// The broken rule.
    pub rule: ContrastRule,
    /// The ratio the scheme has.
    pub ratio: f32,
    /// The ratio it needs.
    pub required: f32,
}

/// Text pairs on fills: every `Role`/`OnRole` pair of [`ColorRole::on_role`].
fn fill_pairs() -> Vec<ContrastRule> {
    ColorRole::all()
        .iter()
        .filter_map(|&bg| {
            bg.on_role().map(|fg| ContrastRule {
                fg,
                bg,
                kind: RuleKind::Text,
            })
        })
        .collect()
}

/// All rules: fill pairs; body and secondary text and the accent/status colours as text on every
/// surface; the outline against every surface.
pub fn rules() -> Vec<ContrastRule> {
    let mut v = fill_pairs();
    for bg in SURFACES {
        for fg in [
            ColorRole::OnSurface,
            ColorRole::OnSurfaceVariant,
            ColorRole::Primary,
            ColorRole::Error,
            ColorRole::Success,
            ColorRole::Warning,
            ColorRole::Info,
        ] {
            // OnSurfaceVariant is only specified for Surface/SurfaceVariant and the containers;
            // all surfaces are listed, which is the stricter reading.
            v.push(ContrastRule {
                fg,
                bg,
                kind: RuleKind::Text,
            });
        }
        v.push(ContrastRule {
            fg: ColorRole::Outline,
            bg,
            kind: RuleKind::Control,
        });
    }
    v
}

/// Every broken rule of `scheme` (empty when the scheme is sound).
pub fn audit(scheme: &ColorScheme) -> Vec<Violation> {
    rules()
        .into_iter()
        .filter_map(|rule| {
            let required = match rule.kind {
                RuleKind::Text => text_min(scheme.kind),
                RuleKind::Control => control_min(scheme.kind),
            };
            let ratio = contrast_ratio(scheme.get(rule.fg), scheme.get(rule.bg));
            (ratio + 1e-4 < required).then_some(Violation {
                rule,
                ratio,
                required,
            })
        })
        .collect()
}

/// Repairs `scheme` in place: for each violated rule the **foreground** role is moved in
/// lightness (away from its background) until the rule holds. Returns the roles that changed.
///
/// Fixing one rule can in principle break another that shares the role, so the pass repeats
/// (at most 4 times). Rules whose foreground cannot reach the ratio even at pure black/white are
/// reported by a final [`audit`] call by the caller.
pub fn auto_correct(scheme: &mut ColorScheme) -> Vec<ColorRole> {
    let mut changed = Vec::new();
    for _ in 0..4 {
        let violations = audit(scheme);
        if violations.is_empty() {
            break;
        }
        for v in violations {
            let bg_colors: Vec<Color> = rules()
                .iter()
                .filter(|r| r.fg == v.rule.fg && r.kind == v.rule.kind)
                .map(|r| scheme.get(r.bg))
                .collect();
            let lighter = scheme.is_dark();
            let fixed = ensure_contrast(scheme.get(v.rule.fg), &bg_colors, v.required, lighter);
            // The search direction guess can be wrong (e.g. OnPrimary on a mid-tone fill):
            // try the other direction when this one did not reach the ratio.
            let fixed = if bg_colors
                .iter()
                .all(|&bg| contrast_ratio(fixed, bg) >= v.required)
            {
                fixed
            } else {
                ensure_contrast(scheme.get(v.rule.fg), &bg_colors, v.required, !lighter)
            };
            scheme.set(v.rule.fg, fixed);
            if !changed.contains(&v.rule.fg) {
                changed.push(v.rule.fg);
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::accent::AccentPreset;
    use crate::theme::mode::ThemeKind;

    const KINDS: [ThemeKind; 3] = [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast];

    #[test]
    fn every_theme_and_preset_passes_every_pair() {
        let mut checked = 0;
        for kind in KINDS {
            for preset in AccentPreset::ALL {
                let scheme = ColorScheme::for_kind(kind, preset.color());
                let violations = audit(&scheme);
                assert!(
                    violations.is_empty(),
                    "{kind:?} / {preset:?}: {} violations, first {:?}",
                    violations.len(),
                    violations.first()
                );
                checked += rules().len();
            }
        }
        assert!(checked > 1000, "{checked} pairs checked");
    }

    #[test]
    fn the_default_constructors_are_sound() {
        for s in [
            ColorScheme::dark(),
            ColorScheme::light(),
            ColorScheme::high_contrast(),
        ] {
            assert!(audit(&s).is_empty(), "{:?}", audit(&s).first());
        }
    }

    #[test]
    fn the_old_amber_pair_fails_and_is_corrected() {
        // light theme, raw amber primary with white text: 2.15:1 (audit finding UI-F09)
        let mut s = ColorScheme::light();
        s.set(ColorRole::Primary, Color::srgb(0.96, 0.62, 0.04));
        s.set(ColorRole::OnPrimary, Color::WHITE);
        assert!(audit(&s)
            .iter()
            .any(|v| v.rule.bg == ColorRole::Primary && v.ratio < 3.0));
        let changed = auto_correct(&mut s);
        assert!(changed.contains(&ColorRole::OnPrimary) || changed.contains(&ColorRole::Primary));
        assert!(audit(&s).is_empty(), "{:?}", audit(&s).first());
    }

    #[test]
    fn high_contrast_reaches_seven_to_one_for_body_text() {
        let s = ColorScheme::high_contrast();
        for bg in SURFACES {
            assert!(contrast_ratio(s.get(ColorRole::OnSurface), s.get(bg)) >= 7.0);
            assert!(contrast_ratio(s.get(ColorRole::Outline), s.get(bg)) >= 4.5);
        }
    }
}
