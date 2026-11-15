//! The scenery settings: draw distance, shadow distances and detail, and the sun's shadow cascades.

use bevy::prelude::*;

use crate::settings::GameSettings;

/// Metres of draw distance per step of `graphics.view_distance`.
pub const VIEW_DISTANCE_STEP_M: f32 = 150.0;
/// The step that means *unlimited* (the largest one).
pub const VIEW_DISTANCE_UNLIMITED: i64 = 8;

/// The scenery draw distance for a `graphics.view_distance` value: 150 m per step (the default 4 is
/// 600 m, beyond the default 3 x 3 tiles); the largest step draws everything that is loaded. A
/// value that is not a number counts as the default, one outside 1 to 8 is clamped.
#[must_use]
pub fn draw_distance_of(settings: &GameSettings) -> Option<f32> {
    let step = settings
        .raw("graphics.view_distance")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(4);
    let step = step.clamp(1, VIEW_DISTANCE_UNLIMITED);
    (step < VIEW_DISTANCE_UNLIMITED).then_some(step as f32 * VIEW_DISTANCE_STEP_M)
}

/// `graphics.view_distance`: how far scenery objects are drawn (M-0029).
pub fn sync_view_distance(
    settings: Res<GameSettings>,
    distance: Option<ResMut<opts_bevy::SceneryDrawDistance>>,
) {
    let Some(mut distance) = distance else { return };
    let want = draw_distance_of(&settings);
    if distance.metres != want {
        distance.metres = want;
    }
}

/// Metres of the sun's shadow cascades when the setting is missing, and the range the setting allows.
pub const SHADOW_DISTANCE_DEFAULT_M: i64 = 300;
/// Radius of the lamps that get a light when the setting is missing (m).
pub const LIGHT_DISTANCE_DEFAULT_M: i64 = 350;
/// LOD detail distance when the setting is missing (percent: 100 is the pure screen-size rule).
pub const DETAIL_DISTANCE_DEFAULT_PERCENT: i64 = 200;
/// Shadow detail when the setting is missing: 0, every scenery part casts (M-0029: on by default
/// only once a measurement shows a frame-time gain, `lod-culling.md` §4).
pub const SHADOW_DETAIL_DEFAULT: i64 = 0;

/// The shadow-culling factor for `graphics.shadow_detail`: metres of cast distance per metre of
/// part radius, `None` (every part casts) for 0.
#[must_use]
pub fn shadow_detail_of(settings: &GameSettings) -> Option<f32> {
    let k = number(
        settings,
        "graphics.shadow_detail",
        SHADOW_DETAIL_DEFAULT,
        0,
        400,
    );
    (k > 0.0).then_some(k)
}

/// `graphics.shadow_detail`: how far small scenery parts cast the sun's shadow (M-0029).
pub fn sync_shadow_detail(
    settings: Res<GameSettings>,
    detail: Option<ResMut<opts_bevy::ShadowDetail>>,
) {
    let Some(mut detail) = detail else { return };
    let factor = shadow_detail_of(&settings);
    if detail.factor != factor {
        detail.factor = factor;
    }
}

fn number(settings: &GameSettings, key: &str, default: i64, lo: i64, hi: i64) -> f32 {
    settings
        .raw(key)
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(default)
        .clamp(lo, hi) as f32
}

/// The shadow cascade configuration for a maximum distance: four cascades, the first reaching a
/// little farther the farther the last one does.
#[must_use]
pub fn shadow_cascades(maximum_distance: f32) -> bevy::light::CascadeShadowConfig {
    bevy::light::CascadeShadowConfigBuilder {
        num_cascades: 4,
        minimum_distance: 0.1,
        maximum_distance,
        first_cascade_far_bound: 5.0 * (maximum_distance / 150.0).sqrt().max(1.0),
        overlap_proportion: 0.2,
    }
    .build()
}

/// `graphics.shadow_distance`, `graphics.light_distance` and `graphics.detail_distance`: how far
/// the sun's shadows reach, how far lamps get lights and how far the finer LOD groups of scenery
/// objects are kept.
pub fn sync_detail_distances(
    settings: Res<GameSettings>,
    lod: Option<ResMut<opts_bevy::SceneryLodBias>>,
    lights: Option<ResMut<opts_bevy::scenery_runtime::MapLightSettings>>,
    mut cascades: Query<&mut bevy::light::CascadeShadowConfig, With<DirectionalLight>>,
) {
    let shadow = number(
        &settings,
        "graphics.shadow_distance",
        SHADOW_DISTANCE_DEFAULT_M,
        50,
        2000,
    );
    for mut c in &mut cascades {
        if c.bounds.last().is_none_or(|b| (*b - shadow).abs() > 0.5) {
            *c = shadow_cascades(shadow);
        }
    }
    if let Some(mut lights) = lights {
        let radius = number(
            &settings,
            "graphics.light_distance",
            LIGHT_DISTANCE_DEFAULT_M,
            50,
            2000,
        );
        if (lights.radius_m - radius).abs() > 0.5 {
            lights.radius_m = radius;
        }
    }
    if let Some(mut lod) = lod {
        let bias = number(
            &settings,
            "graphics.detail_distance",
            DETAIL_DISTANCE_DEFAULT_PERCENT,
            25,
            800,
        ) / 100.0;
        if (lod.0 - bias).abs() > 1e-3 {
            lod.0 = bias;
        }
    }
}
