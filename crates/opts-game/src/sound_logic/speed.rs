//! The speed of a loop or a plain sound, and the level and speed of a loop.

use super::*;

/// The slowest a loop plays: below it the loop is silent. A loop whose pitch variable is 0 stops
/// (the warning loops `brake_warning` and `notheben_initialized` of the MAN buses play at
/// `1 + (0 / 1 - 1) × 1 = 0` while nothing is wrong; the engine loops of a stopped engine, `engine_n`
/// 0, likewise): they have no `[conditionSingle]` and no volume curve, the pitch is their switch.
pub const MIN_SPEED: f32 = 0.1;

/// Level and speed of a loop over `read`: silent (level 0) while the speed is below
/// [`MIN_SPEED`], else its volume curves' level and the speed.
pub fn loop_voice(d: &SoundDef, read: impl Fn(&str) -> f32 + Copy) -> (f32, f32) {
    let s = speed(d, read);
    if s < MIN_SPEED {
        (0.0, MIN_SPEED)
    } else {
        (level(d, read), s)
    }
}

/// The playback speed: a loop plays at its natural speed when the variable equals the reference
/// value and follows it with a sensitivity, `1 + (variable / reference - 1) × scale` (limited to
/// 3.5 above, and not limited below: a speed at or below zero is no sound, see [`loop_voice`]; with the scale 1 that is `variable / reference`, the engine samples named by their
/// rpm); a plain sound its pitch factor. The scale is *not* a factor on the whole ratio: the
/// documented reference value is the natural pitch whatever the scale is (the Citybus 530's
/// engine loops have the scale 2, the MAN's exterior loops 0.8, its turbo loops 0.05; a factor
/// on the ratio would play the Citybus' idle loop an octave too high; ❓ Q-SND-001).
pub fn speed(d: &SoundDef, read: impl Fn(&str) -> f32) -> f32 {
    match (d.kind, d.pitch_variable.as_deref()) {
        (SoundKind::Loop, Some(v)) if d.pitch_reference.abs() > f32::EPSILON => {
            (1.0 + (read(v) / d.pitch_reference - 1.0) * d.pitch_scale).min(3.5)
        }
        (SoundKind::Loop, _) => 1.0,
        (SoundKind::Sound, _) => d.pitch.clamp(0.3, 3.5),
    }
}
