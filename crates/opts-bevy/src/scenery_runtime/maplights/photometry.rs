//! Photometry of a lamp (lumens and range from its radius) and the nearest-lamp choice.

use bevy::prelude::*;

/// Luminous power (lm) of a lamp with full-intensity radius `radius_m` and full illuminance
/// `full_lux`: `4π·r²·E`.
#[must_use]
pub fn map_light_lumens(radius_m: f32, full_lux: f32) -> f32 {
    4.0 * std::f32::consts::PI * radius_m * radius_m * full_lux
}

/// Range of a lamp's point light (m): `8·r`.
#[must_use]
pub fn map_light_range(radius_m: f32) -> f32 {
    8.0 * radius_m
}

/// The lamps that should hold a light: the `n` nearest of `candidates` (lamp, distance) within
/// `radius_m`, where lamps in `current` count `hysteresis_m` nearer than they are. Ties break by
/// entity, so the choice is stable from one assignment to the next.
#[must_use]
pub fn choose_nearest(
    candidates: &[(Entity, f32)],
    current: &[Entity],
    n: usize,
    radius_m: f32,
    hysteresis_m: f32,
) -> Vec<Entity> {
    let mut ranked: Vec<(f32, Entity)> = candidates
        .iter()
        .filter_map(|&(e, d)| {
            let held = current.contains(&e);
            let reach = if held {
                radius_m + hysteresis_m
            } else {
                radius_m
            };
            (d <= reach).then_some((if held { d - hysteresis_m } else { d }, e))
        })
        .collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    ranked.into_iter().take(n).map(|(_, e)| e).collect()
}
