//! Vehicles without a script engine animate from a plain variable table (AI traffic).

use bevy::prelude::*;
use opts_bevy::{
    vehicle_animation_system, vehicle_light_map_system, AiVariables, AnimStep, LightMapBinding,
    MeshAnimation,
};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.init_resource::<Assets<StandardMaterial>>();
    app.add_systems(Update, (vehicle_animation_system, vehicle_light_map_system));
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
    assert_eq!(emissive(&app, part), LinearRgba::rgb(1.0, 1.0, 1.0));
    // The shared material was copied, not changed.
    let shared = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .get(&handle)
        .unwrap()
        .emissive;
    assert_eq!(shared, StandardMaterial::default().emissive);
}
