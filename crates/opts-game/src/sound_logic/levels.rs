//! The comparisons of conditions and the level of a definition over the script variables.

use super::*;

/// A comparison of `mode` between a variable and a condition value: `None` for a mode that is
/// not understood.
#[must_use]
pub fn compare(mode: f32, variable: f32, value: f32) -> Option<bool> {
    match mode.round() as i32 {
        0 => Some((variable - value).abs() >= 0.05),
        1 => Some((variable - value).abs() < 0.05),
        2 => Some(variable < value),
        3 => Some(variable > value),
        4 => Some(variable >= value - 0.05),
        5 => Some(variable <= value + 0.05),
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
