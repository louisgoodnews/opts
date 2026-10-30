//! Vehicles without a script engine animate from a plain variable table (AI traffic).

use bevy::prelude::*;
use opts_bevy::{
    vehicle_alpha_scale_system, vehicle_animation_system, vehicle_light_map_system, AiVariables,
    AlphaScaleBinding, AnimStep, LightMapBinding, MeshAnimation,
};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(
        Update,
        (
            vehicle_animation_system,
            vehicle_light_map_system,
            vehicle_alpha_scale_system,
        ),
    );
    app
}

#[test]
fn a_wheel_turns_with_its_variable_and_a_visible_rule_follows_it() {
    let mut app = app();
    let owner = app.world_mut().spawn(AiVariables::default()).id();
    let wheel = app
        .world_mut()
        .spawn((
            MeshAnimation {
                owner,
                steps: vec![AnimStep {
                    frame: Mat4::IDENTITY,
                    variable: "Wheel_Rotation_0_L".into(),
                    factor: 57.295_78,
                    translate: false,
                    maxspeed: None,
                    delay: None,
                }],
                visible: Vec::new(),
                shown: Vec::new(),
            },
            Transform::IDENTITY,
            Visibility::Inherited,
        ))
        .id();
    let passenger = app
        .world_mut()
        .spawn((
            MeshAnimation {
                owner,
                steps: Vec::new(),
                visible: vec![("ai_pax2".into(), 1.0)],
                shown: Vec::new(),
            },
            Transform::IDENTITY,
            Visibility::Inherited,
        ))
        .id();
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(passenger).unwrap(),
        Visibility::Hidden
    );
    assert!(
        app.world()
            .get::<Transform>(wheel)
            .unwrap()
            .rotation
            .angle_between(Quat::IDENTITY)
            < 1e-6
    );

    let mut vars = app.world_mut().get_mut::<AiVariables>(owner).unwrap();
    vars.set("wheel_rotation_0_l", 1.0);
    vars.set("ai_pax2", 1.0);
    app.update();
    let angle = app
        .world()
        .get::<Transform>(wheel)
        .unwrap()
        .rotation
        .angle_between(Quat::IDENTITY);
    assert!((angle - 1.0).abs() < 1e-4, "wheel turned {angle} rad");
    assert_eq!(
        *app.world().get::<Visibility>(passenger).unwrap(),
        Visibility::Inherited
    );
}

#[test]
fn a_light_map_glows_with_ai_light() {
    let mut app = app();
    let owner = app.world_mut().spawn(AiVariables::default()).id();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial::default());
    let part = app
        .world_mut()
        .spawn((
            LightMapBinding::new(owner, "ai_light"),
            MeshMaterial3d(handle.clone()),
        ))
        .id();
    app.update();
    let emissive = |app: &App, e: Entity| {
        let h = &app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(e)
            .unwrap()
            .0;
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(h)
            .unwrap()
            .emissive
    };
    assert_eq!(emissive(&app, part), LinearRgba::BLACK);
    app.world_mut()
        .get_mut::<AiVariables>(owner)
        .unwrap()
        .set("ai_light", 1.0);
    app.update();
    let g = opts_bevy::vehicle_lights::LIGHT_MAP_GAIN;
    assert_eq!(emissive(&app, part), LinearRgba::rgb(g, g, g));
    // The shared material was copied, not changed.
    let shared = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .get(&handle)
        .unwrap()
        .emissive;
    assert_eq!(shared, StandardMaterial::default().emissive);
}

/// MAN SD83: window glass (`Fenster.tga`, alpha 2, `[alphascale] Envir_Brightness`) is hidden at
/// night-time brightness 0, tinted in proportion to the brightness by day, on a private copy.
#[test]
fn glass_follows_its_alpha_scale_variable() {
    let mut app = app();
    let owner = app.world_mut().spawn(AiVariables::default()).id();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgba(0.0, 0.0, 0.0, 0.5),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
    let glass = app
        .world_mut()
        .spawn((
            AlphaScaleBinding::new(owner, "Envir_Brightness"),
            MeshMaterial3d(handle.clone()),
            Visibility::Hidden,
        ))
        .id();
    let alpha = |app: &App| {
        let h = &app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(glass)
            .unwrap()
            .0;
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(h)
            .unwrap()
            .base_color
            .alpha()
    };
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(glass).unwrap(),
        Visibility::Hidden,
        "variable 0: no glass"
    );
    app.world_mut()
        .get_mut::<AiVariables>(owner)
        .unwrap()
        .set("envir_brightness", 1.0);
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(glass).unwrap(),
        Visibility::Inherited
    );
    assert!((alpha(&app) - 0.5).abs() < 1e-6);
    app.world_mut()
        .get_mut::<AiVariables>(owner)
        .unwrap()
        .set("envir_brightness", 0.4);
    app.update();
    assert!((alpha(&app) - 0.2).abs() < 1e-6, "base alpha 0.5 x 0.4");
    // The shared material is untouched.
    let shared = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .get(&handle)
        .unwrap()
        .base_color
        .alpha();
    assert!((shared - 0.5).abs() < 1e-6);
}

#[test]
fn a_parked_car_body_follows_the_daylight_and_other_variables_stay_out() {
    let mut app = app();
    // Noon on a summer day: the sun is high, `Envir_Brightness` is 1.
    app.insert_resource(opts_bevy::SceneryClock::at(2026, 6, 21, 12.0 * 3600.0));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial::default());
    // A vehicle model placed as scenery: no scripts, no AI table.
    let owner = app.world_mut().spawn(Transform::default()).id();
    let mut part = |var: &str| {
        app.world_mut()
            .spawn((
                AlphaScaleBinding::new(owner, var),
                MeshMaterial3d(material.clone()),
                Visibility::Hidden,
            ))
            .id()
    };
    let body = part("Envir_Brightness");
    let dirt = part("Dirt_Norm");
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(body).unwrap(),
        Visibility::Inherited,
        "the body shows by day"
    );
    assert_eq!(
        *app.world().get::<Visibility>(dirt).unwrap(),
        Visibility::Hidden
    );
}
