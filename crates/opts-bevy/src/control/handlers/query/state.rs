//! The state of the session as JSON: the base state, the camera and the simulation.

use super::*;


/// Facts every application has; the game adds app state, map and vehicle on top.
#[must_use]
pub fn base_state(world: &World) -> Map<String, Value> {
    let control = world.resource::<ControlState>();
    let mut m = Map::new();
    m.insert("frame".into(), json!(control.frame));
    m.insert(
        "fps".into(),
        json!((f64::from(control.fps) * 10.0).round() / 10.0),
    );
    let worst = control
        .frame_log
        .iter()
        .map(|(_, ms)| *ms)
        .fold(0.0, f32::max);
    m.insert(
        "frame_ms_max".into(),
        json!((f64::from(worst) * 10.0).round() / 10.0),
    );
    if let Some(cam) = world.get_resource::<CameraState>() {
        m.insert("camera_mode".into(), json!(cam.mode.label()));
    }
    m
}

pub(super) fn state(world: &mut World, _: &Value) -> Outcome {
    Outcome::ok(Value::Object(base_state(world)))
}

pub(super) fn camera_get(world: &mut World, _: &Value) -> Outcome {
    let mut out = Map::new();
    if let Some(cam) = world.get_resource::<CameraState>() {
        out.insert("mode".into(), json!(cam.mode.label()));
        out.insert("sub_index".into(), json!(cam.sub_camera_index));
    }
    let mut query =
        world.query_filtered::<(&GlobalTransform, Option<&Projection>), With<crate::MainCamera>>();
    if let Some((tf, projection)) = query.iter(world).next() {
        let (t, r) = (tf.translation(), tf.to_scale_rotation_translation().1);
        out.insert(
            "transform".into(),
            json!({ "translation": [t.x, t.y, t.z], "rotation": [r.x, r.y, r.z, r.w] }),
        );
        if let Some(Projection::Perspective(p)) = projection {
            out.insert("fov".into(), json!(p.fov.to_degrees()));
        }
    }
    Outcome::ok(Value::Object(out))
}

pub(super) fn sim_get(world: &mut World, _: &Value) -> Outcome {
    let virt = world.resource::<Time<Virtual>>();
    let tick = world.resource::<ControlState>().frame;
    let _ = world.resource::<Time<Real>>();
    let mut out = json!({
        "paused": virt.is_paused(),
        "sim_time": virt.elapsed_secs_f64(),
        "time_scale": virt.relative_speed(),
        "tick": tick,
    });
    // The session clock (sky, traffic, scenery), when the app has one.
    if let Some(env) = world.get_resource::<crate::environment::SessionEnvironment>() {
        out["clock"] = json!(env.clock_text());
        out["seconds_of_day"] = json!(env.seconds_of_day());
        out["date"] = json!(env.date().to_string());
        out["map_date"] = json!(env.map_date().to_string());
        out["weather"] = json!(env.weather());
        out["clock_rate"] = json!(env.rate());
        out["weather_conditions"] = weather_json(env);
        if let Some(view) = world.get_resource::<crate::weather::GroundFogView>() {
            let w = &mut out["weather_conditions"];
            w["fog_layer_m"] = json!(view.layer_m);
            w["fog_effective_visibility_m"] = json!(view.effective_visibility_m);
            w["fog_camera_height_m"] = json!(view.height_m);
            w["fog_sky_clear"] = json!(view.sky_clear);
        }
        if let Some(cat) = world.get_resource::<crate::weather::WeatherCatalogue>() {
            out["weather_canonical"] = json!(cat.canonical_id(env.weather()));
            out["weather_presets"] = json!(cat.ids());
        }
    }
    if let Some(q) = world.get_resource::<crate::light_quality::LightQuality>() {
        out["light_quality"] = light_quality_json(q);
        if let Some(g) = world.get_resource::<crate::light_quality::GovernorState>() {
            out["light_quality"]["target_ms"] = json!(g.target_ms);
            out["light_quality"]["late_share"] = json!(g.late_share);
            out["light_quality"]["up_wait_s"] = json!(g.up_wait_s);
        }
        if let Some(b) = world.get_resource::<crate::light_quality::GovernorBudget>() {
            out["light_quality"]["vsync"] = json!(b.vsync);
        }
    }
    out["reflections"] = reflections_json(world);
    if let Some(c) = world.get_resource::<crate::light_cones::ConeStats>() {
        out["light_cones"] = json!({
            "total": c.total,
            "eligible": c.eligible,
            "active": c.active,
            "hidden_by_budget": c.over_budget,
            "select_us": c.select_us,
        });
    }
    Outcome::ok(out)
}
