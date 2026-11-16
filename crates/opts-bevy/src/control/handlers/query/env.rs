//! The environment as JSON: reflections, light quality and weather.

use super::*;

/// The reflection features as they run (what `sim.get` reports as `reflections`): the shared sky
/// cube, the glass look and the mirror cameras.
pub(super) fn reflections_json(world: &World) -> Value {
    let mut out = json!({});
    if let Some(c) = world.get_resource::<crate::sky_cube::SkyCubeState>() {
        out["sky_cube"] = json!({
            "on": c.handle.is_some(),
            "builds": c.builds,
            "intensity": c.intensity,
        });
    }
    if let Some(g) = world.get_resource::<crate::glass::GlassStats>() {
        out["glass"] = json!({"registered": g.registered, "reflective": g.reflective});
    }
    if let Some(m) = world.get_resource::<crate::mirrors::MirrorStats>() {
        out["mirrors"] = json!({
            "cameras": m.cameras,
            "selected": m.selected,
            "rendering": m.rendering,
            "ambient": m.ambient,
        });
    }
    out
}

/// The lighting quality as the effects see it (what `sim.get` reports as `light_quality`).
pub(super) fn light_quality_json(q: &crate::light_quality::LightQuality) -> Value {
    json!({
        "mode": q.mode.name(),
        "level": q.level,
        "level_name": crate::light_quality::level_name(q.level),
        "values": {
            "shafts": q.shafts.name(),
            "shaft_steps": q.shafts.steps(),
            "night_shafts": q.night_shafts,
            "lantern_cones": q.lantern_cones,
            "headlight_cones": q.headlight_cones,
            "cone_cap": q.cone_cap,
            "cone_depth_fade": q.cone_depth_fade,
            "mirror_count": q.mirror_count,
            "mirror_every": q.mirror_every,
            "glass_reflections": q.glass_reflections,
            "bloom": q.bloom.name(),
        },
    })
}

/// The resolved weather as the effects see it (what `sim.get` reports as `weather_conditions`).
pub(super) fn weather_json(env: &crate::environment::SessionEnvironment) -> Value {
    let c = env.conditions();
    json!({
        "fog_visibility_m": c.fog_visibility_m,
        "wind_from_deg": c.wind_from_deg,
        "wind_speed": c.wind_speed,
        "temperature_c": c.temperature_c,
        "dew_point_c": c.dew_point_c,
        "pressure_hpa": c.pressure_hpa,
        "clouds": c.cloud.as_ref().map(|cl| json!({"type": cl.name, "base_m": cl.base_m})),
        "precip_type": c.precip.code(),
        "precip_rate": c.precip_rate,
        "wetness": crate::weather::surface_state(c).wet,
        "snow": c.snow,
        "sun_scale": env.attenuation(),
    })
}
