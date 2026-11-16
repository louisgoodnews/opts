//! A light map glows with AI light.

use super::*;

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
