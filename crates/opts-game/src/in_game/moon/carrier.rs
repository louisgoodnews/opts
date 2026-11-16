//! The shadow carrier: which directional light (sun or moon) casts the shadows, with hysteresis.

use super::*;

/// Relative margin by which the other light must beat the carrier to take over the shadows.
pub const CARRIER_HYSTERESIS: f32 = 0.15;
/// Illuminance (lx) below which a directional light is hidden and cannot carry shadows.
pub const MIN_LIGHT_LX: f32 = 1.0;

/// The light that currently carries the shadows (`None` when neither gives light).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShadowCarrier(pub Option<LightRole>);

/// The carrier after one step. `sun` and `moon` are the scores (illuminance, 0 when the light is
/// below the horizon). The current carrier is kept until the other light beats it by
/// [`CARRIER_HYSTERESIS`] (or the current one falls below half of [`MIN_LIGHT_LX`]); with no
/// carrier, or on a tie, the larger score wins and the sun wins ties.
#[must_use]
pub fn choose_carrier(current: Option<LightRole>, sun: f32, moon: f32) -> Option<LightRole> {
    let score = |r: LightRole| match r {
        LightRole::Sun => sun,
        LightRole::Moon => moon,
    };
    let other = |r: LightRole| match r {
        LightRole::Sun => LightRole::Moon,
        LightRole::Moon => LightRole::Sun,
    };
    match current {
        Some(cur) if score(cur) >= MIN_LIGHT_LX * 0.5 => {
            let o = other(cur);
            if score(o) >= MIN_LIGHT_LX && score(o) > score(cur) * (1.0 + CARRIER_HYSTERESIS) {
                Some(o)
            } else {
                Some(cur)
            }
        }
        _ => {
            let best = if moon > sun {
                LightRole::Moon
            } else {
                LightRole::Sun
            };
            (score(best) >= MIN_LIGHT_LX).then_some(best)
        }
    }
}

/// Keeps exactly one directional light with shadow maps ([`choose_carrier`]) and hides the
/// lights that give no light (a hidden light is not extracted for shadow work).
pub fn shadow_carrier_system(
    env: Res<SessionEnvironment>,
    mut carrier: ResMut<ShadowCarrier>,
    mut lights: Query<(&LightRole, &mut DirectionalLight, &mut Visibility)>,
) {
    let score = |role: LightRole, light: &DirectionalLight| {
        let up = match role {
            LightRole::Sun => env.sun_alt_deg() > 0.0,
            LightRole::Moon => env.moon().elevation_deg > 0.0,
        };
        if up {
            light.illuminance
        } else {
            0.0
        }
    };
    let (mut sun, mut moon) = (0.0_f32, 0.0_f32);
    for (role, light, _) in &lights {
        match role {
            LightRole::Sun => sun = sun.max(score(*role, light)),
            LightRole::Moon => moon = moon.max(score(*role, light)),
        }
    }
    let next = choose_carrier(carrier.0, sun, moon);
    if carrier.0 != next {
        carrier.0 = next;
    }
    for (role, mut light, mut visibility) in &mut lights {
        let shadows = next == Some(*role);
        if light.shadow_maps_enabled != shadows {
            light.shadow_maps_enabled = shadows;
        }
        let want = if light.illuminance < MIN_LIGHT_LX {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *visibility != want {
            *visibility = want;
        }
    }
}
