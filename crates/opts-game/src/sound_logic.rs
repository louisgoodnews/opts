//! When a sound definition plays (`docs/src/formats/text/vehicles/sound_cfg.md`): shared by the
//! scenery, AI and player-vehicle layers so that all three read a configuration the same way.
//!
//! A definition is **active** when its `[conditionSingle]`s hold (all of them), or always when it
//! has none. What activation does depends on its keywords:
//!
//! | keywords | plays |
//! | :--- | :--- |
//! | `[loopsound]`, or `[sound]`, no trigger, no `[noloop]` | as a loop while active |
//! | `[noloop]` and conditions, no trigger | once, when it becomes active |
//! | `[trigger]` | once per raised event (while active, when it has conditions) |
//!
//! The comparison of a `[conditionSingle] variable value mode` is read from usage (13,270
//! conditions in the installed vehicles; ❓ Q-SND-003): mode 1 is "equals" (`engine_on 0`,
//! `antrieb_wendesatz 1`, `wiperrunning 1`: 9,659), mode 2 is "below" (`velocity 1`,
//! `throttle 0.1`, `brake 0.02`: stopped, off the pedal), mode 3 is "above" (`engine_n 300`,
//! `engine_n 200`, `velocity 3`: engine running, driving). Modes 0, 4, 5 and -1 (630 uses, among
//! them the horn `cockpit_hupe_volume 3 0` and the retarder stages) are not understood and make a
//! definition [`Playback::Unsupported`].

use opts_core::formats::sound::{Condition, SoundDef, SoundKind};

/// What a definition does when it is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Playback {
    /// A loop while active.
    Loop,
    /// Once, when the conditions turn true.
    OnEdge,
    /// Once per raised event.
    OnTrigger,
    /// Uses a condition mode that is not understood.
    Unsupported,
}

/// Reads a definition's keywords into what it does.
#[must_use]
pub fn playback_of(d: &SoundDef) -> Playback {
    if d.conditions
        .iter()
        .any(|c| compare(c.mode, 0.0, 0.0).is_none())
    {
        return Playback::Unsupported;
    }
    if !d.triggers.is_empty() {
        return Playback::OnTrigger;
    }
    if !d.conditions.is_empty() && d.noloop {
        return Playback::OnEdge;
    }
    if d.kind == SoundKind::Loop || !d.noloop {
        return Playback::Loop;
    }
    // `[noloop]` alone: nothing starts it.
    Playback::Unsupported
}

/// A comparison of `mode` between a variable and a condition value: `None` for a mode that is
/// not understood.
#[must_use]
pub fn compare(mode: f32, variable: f32, value: f32) -> Option<bool> {
    match mode.round() as i32 {
        1 => Some((variable - value).abs() < 0.05),
        2 => Some(variable < value),
        3 => Some(variable > value),
        _ => None,
    }
}

/// Whether all conditions of a definition hold over `read` (variables by name); `None` when one
/// of them has a mode that is not understood.
pub fn conditions_hold(conditions: &[Condition], read: impl Fn(&str) -> f32) -> Option<bool> {
    let mut all = true;
    for c in conditions {
        all &= compare(c.mode, read(&c.variable), c.value)?;
    }
    Some(all)
}

/// The "variable" of a volume curve that is no script variable: `-1` is the time the sound has
/// been playing (seconds), which the starter and retarder loops use to fade in (`[pnt] 0.1 0`,
/// `[pnt] 0.3 1`; ❓ Q-SND-005, read from usage: nothing else rises with the start of a sound).
pub const AGE_VARIABLE: &str = "-1";

/// The level of a definition: the product of its volume curves over `read`.
pub fn level(d: &SoundDef, read: impl Fn(&str) -> f32) -> f32 {
    d.volcurves
        .iter()
        .map(|c| c.at(read(&c.variable)))
        .product::<f32>()
        .clamp(0.0, 1.0)
}

/// The playback speed: a loop plays at its natural speed when the variable equals the reference
/// value and follows it with a sensitivity, `1 + (variable / reference - 1) × scale` (limited to
/// 0.3 to 3.5; with the scale 1 that is `variable / reference`, the engine samples named by their
/// rpm); a plain sound its pitch factor. The scale is *not* a factor on the whole ratio: the
/// documented reference value is the natural pitch whatever the scale is (the Citybus 530's
/// engine loops have the scale 2, the MAN's exterior loops 0.8, its turbo loops 0.05; a factor
/// on the ratio would play the Citybus' idle loop an octave too high; ❓ Q-SND-001).
pub fn speed(d: &SoundDef, read: impl Fn(&str) -> f32) -> f32 {
    match (d.kind, d.pitch_variable.as_deref()) {
        (SoundKind::Loop, Some(v)) if d.pitch_reference.abs() > f32::EPSILON => {
            (1.0 + (read(v) / d.pitch_reference - 1.0) * d.pitch_scale).clamp(0.3, 3.5)
        }
        (SoundKind::Loop, _) => 1.0,
        (SoundKind::Sound, _) => d.pitch.clamp(0.3, 3.5),
    }
}

/// Which camera a `[viewpoint]` allows: the interior sounds have 2 (and 6), the exterior ones 5 (and
/// 1); every other value (and none) plays everywhere. Read from the file names: in the installed
/// vehicles 5,586 of the 11,163 definitions with 5 are named `ext`, `außen` ... (the Citybus 530's
/// `OM_906hLA_E2_ext_*` engine loops and `rear_piep_piep_ext` have 5, the engine loops without
/// `ext` have 2), only 29 of the 13,033 with 2 are; 88 of the 96 with 1 are named `eng_out_*` or
/// the like, 28 of the 38 with 6 `engine_in_*`. ❓ Q-SND-004: the values 3 and 4 are unknown.
#[must_use]
pub fn viewpoint_allows(viewpoint: Option<i32>, camera_inside: bool) -> bool {
    match viewpoint {
        Some(2 | 6) => camera_inside,
        Some(1 | 5) => !camera_inside,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_core::formats::sound::SoundConfig;

    fn def(text: &str) -> SoundDef {
        SoundConfig::parse(text).sounds.remove(0)
    }

    #[test]
    fn keywords_decide_what_a_definition_does() {
        let d3 = "[3d]\n0\n0\n0\n5\n";
        assert_eq!(
            playback_of(&def(&format!("[sound]\nhum.wav\n1\n{d3}"))),
            Playback::Loop
        );
        assert_eq!(
            playback_of(&def(&format!(
                "[loopsound]\nm.wav\n44100\nengine_n\n600\n1\n{d3}"
            ))),
            Playback::Loop
        );
        assert_eq!(
            playback_of(&def(&format!("[sound]\nbell.wav\n1\n{d3}[trigger]\nev\n"))),
            Playback::OnTrigger
        );
        assert_eq!(
            playback_of(&def(&format!(
                "[sound]\nk.wav\n1\n{d3}[conditionSingle]\nfill\n1\n1\n[noloop]\n"
            ))),
            Playback::OnEdge
        );
        assert_eq!(
            playback_of(&def(&format!(
                "[sound]\nbeep.wav\n1\n{d3}[conditionSingle]\nalert\n1\n1\n"
            ))),
            Playback::Loop
        );
        assert_eq!(
            playback_of(&def(&format!(
                "[sound]\nhorn.wav\n1\n{d3}[conditionSingle]\nhupe\n3\n0\n"
            ))),
            Playback::Unsupported
        );
        assert_eq!(
            playback_of(&def(&format!("[sound]\nx.wav\n1\n{d3}[noloop]\n"))),
            Playback::Unsupported
        );
    }

    #[test]
    fn the_modes_compare_as_equal_below_and_above() {
        assert_eq!(compare(1.0, 1.0, 1.0), Some(true));
        assert_eq!(compare(1.0, 0.0, 1.0), Some(false));
        assert_eq!(compare(2.0, 0.5, 1.0), Some(true));
        assert_eq!(compare(2.0, 2.0, 1.0), Some(false));
        assert_eq!(compare(3.0, 700.0, 300.0), Some(true));
        assert_eq!(compare(3.0, 100.0, 300.0), Some(false));
        for m in [0.0, 4.0, 5.0, -1.0] {
            assert_eq!(compare(m, 1.0, 1.0), None);
        }
    }

    #[test]
    fn all_conditions_must_hold() {
        let d = def("[sound]\ns.wav\n1\n[3d]\n0\n0\n0\n5\n[conditionSingle]\nengine_n\n300\n3\n[conditionSingle]\nvelocity\n1\n2\n");
        let read = |engine_n: f32, v: f32| {
            move |n: &str| match n {
                "engine_n" => engine_n,
                _ => v,
            }
        };
        assert_eq!(conditions_hold(&d.conditions, read(800.0, 0.5)), Some(true));
        assert_eq!(
            conditions_hold(&d.conditions, read(800.0, 5.0)),
            Some(false)
        );
        assert_eq!(
            conditions_hold(&d.conditions, read(100.0, 0.5)),
            Some(false)
        );
        assert_eq!(conditions_hold(&[], read(0.0, 0.0)), Some(true));
    }

    #[test]
    fn level_and_speed_follow_the_variables() {
        let d = def("[loopsound]\nm.wav\n44100\nengine_n\n600\n1\n[3d]\n0\n0\n0\n5\n[volcurve]\nengine_n\n[pnt]\n400\n0\n[pnt]\n800\n1\n");
        let read = |n: &str| if n == "engine_n" { 600.0 } else { 0.0 };
        assert!((level(&d, read) - 0.5).abs() < 1e-5);
        assert!((speed(&d, read) - 1.0).abs() < 1e-5);
        let fast = |_: &str| 1200.0;
        assert!((speed(&d, fast) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn the_reference_value_is_the_natural_speed_whatever_the_scale() {
        // The Citybus 530's idle loop: reference 600, scale 2. At 598 rpm it plays at about its
        // natural speed, not at twice it; the scale makes the pitch follow the rpm more steeply.
        let d = def("[loopsound]\nm.wav\n44100\nengine_n\n600\n2\n[3d]\n0\n0\n0\n5\n");
        assert!((speed(&d, |_| 598.0) - 1.0).abs() < 0.01);
        assert!(
            (speed(&d, |_| 690.0) - 1.3).abs() < 1e-4,
            "15 % more rpm, 30 % higher"
        );
        // A turbo whine with the scale 0.05 hardly changes its pitch.
        let t = def("[loopsound]\nt.wav\n44100\nengine_turbo_RPM\n28\n0.05\n[3d]\n0\n0\n0\n5\n");
        assert!((speed(&t, |_| 56.0) - 1.05).abs() < 1e-4);
        // Far from the reference the speed is limited.
        assert_eq!(speed(&d, |_| 0.0), 0.3);
        assert_eq!(speed(&d, |_| 5000.0), 3.5);
    }

    #[test]
    fn viewpoints_split_inside_from_outside() {
        assert!(viewpoint_allows(None, true) && viewpoint_allows(None, false));
        // 2 and 6 are the interior sounds, 5 and 1 the exterior ones.
        assert!(viewpoint_allows(Some(2), true) && !viewpoint_allows(Some(2), false));
        assert!(viewpoint_allows(Some(6), true) && !viewpoint_allows(Some(6), false));
        assert!(viewpoint_allows(Some(5), false) && !viewpoint_allows(Some(5), true));
        assert!(viewpoint_allows(Some(1), false) && !viewpoint_allows(Some(1), true));
        assert!(viewpoint_allows(Some(7), true) && viewpoint_allows(Some(7), false));
    }
}
