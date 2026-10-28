//! Unit tests for `super` (`query.rs`).

use super::*;
use crate::environment::{CalendarDate, EnvironmentChange, SessionEnvironment};

#[test]
fn sim_get_reports_the_resolved_weather_and_the_available_presets() {
    let mut world = World::new();
    world.insert_resource(Time::<Virtual>::default());
    world.insert_resource(Time::<Real>::default());
    world.insert_resource(ControlState::default());
    world.insert_resource(crate::weather::WeatherCatalogue::default());
    let mut env = SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 43_200.0, "");
    env.apply(&EnvironmentChange {
        weather: Some("Winter Snowfall".into()),
        ..Default::default()
    });
    world.insert_resource(env);
    let out = sim_get(&mut world, &Value::Null);
    let Outcome::Done(Ok(v)) = out else {
        panic!("sim.get did not answer at once");
    };
    assert_eq!(v["weather"], "Winter Snowfall");
    assert_eq!(v["weather_canonical"], "Winter Snowfall");
    assert_eq!(v["weather_presets"].as_array().map(Vec::len), Some(5));
    let c = &v["weather_conditions"];
    assert_eq!(c["precip_type"], 2);
    assert_eq!(c["snow"], true);
    assert_eq!(c["temperature_c"], -3.0);
    assert!(c["sun_scale"].as_f64().unwrap() < 0.6);
    // The ground-fog layer shows once the game publishes the camera's view of it.
    assert!(c.get("fog_layer_m").is_none());
    world.insert_resource(crate::weather::GroundFogView {
        layer_m: Some(30.0),
        effective_visibility_m: 175.0,
        ..Default::default()
    });
    let Outcome::Done(Ok(v)) = sim_get(&mut world, &Value::Null) else {
        panic!("sim.get did not answer at once");
    };
    let c = &v["weather_conditions"];
    assert_eq!(c["fog_layer_m"], 30.0);
    assert_eq!(c["fog_effective_visibility_m"], 175.0);
}

#[test]
fn sim_get_reports_the_lighting_quality_when_the_app_has_one() {
    let mut world = World::new();
    world.insert_resource(Time::<Virtual>::default());
    world.insert_resource(Time::<Real>::default());
    world.insert_resource(ControlState::default());
    let Outcome::Done(Ok(v)) = sim_get(&mut world, &Value::Null) else {
        panic!("sim.get did not answer at once");
    };
    assert!(v.get("light_quality").is_none());
    world.insert_resource(crate::light_quality::ladder_preset(2));
    let Outcome::Done(Ok(v)) = sim_get(&mut world, &Value::Null) else {
        panic!("sim.get did not answer at once");
    };
    let l = &v["light_quality"];
    assert_eq!(l["mode"], "automatic");
    assert_eq!(l["level"], 2);
    assert_eq!(l["level_name"], "headlights-only");
    assert_eq!(l["values"]["shaft_steps"], 16);
    assert_eq!(l["values"]["cone_cap"], 16);
    assert_eq!(l["values"]["lantern_cones"], false);
    assert_eq!(l["values"]["headlight_cones"], true);
    assert!(l.get("target_ms").is_none());
    world.insert_resource(crate::light_quality::GovernorState::default());
    world.insert_resource(crate::light_quality::GovernorBudget::default());
    let Outcome::Done(Ok(v)) = sim_get(&mut world, &Value::Null) else {
        panic!("sim.get did not answer at once");
    };
    let l = &v["light_quality"];
    assert!((l["target_ms"].as_f64().unwrap() - 16.667).abs() < 0.01);
    assert_eq!(l["late_share"], 0.0);
    assert_eq!(l["vsync"], false);
    assert!(v.get("light_cones").is_none());
    world.insert_resource(crate::light_cones::ConeStats {
        total: 120,
        eligible: 80,
        active: 64,
        over_budget: 16,
        select_us: 30,
    });
    let Outcome::Done(Ok(v)) = sim_get(&mut world, &Value::Null) else {
        panic!("sim.get did not answer at once");
    };
    assert_eq!(v["light_cones"]["active"], 64);
    assert_eq!(v["light_cones"]["hidden_by_budget"], 16);
}
