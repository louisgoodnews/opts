//! Builds the cones of a scenery object or of a vehicle model, headless.

use super::*;

/// Builds the scenery object at `path` (no meshes) and returns its cones and whether the root
/// carries the `[nomaplighting]` marker.
pub(super) fn cones_of_sco(path: &std::path::Path) -> Option<(Vec<LightCone>, bool)> {
    let bytes = std::fs::read(path).ok()?;
    let sco = SceneryObject::parse(&bytes).expect("sco parses");
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            watch_for_changes_override: Some(false),
            ..Default::default()
        },
        bevy::transform::TransformPlugin,
    ))
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<StandardMaterial>>();
    let root = app
        .world_mut()
        .spawn((Transform::default(), Visibility::Inherited))
        .id();
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut materials: ResMut<Assets<StandardMaterial>>| {
                let input = SceneryObjectInput {
                    sco: &sco,
                    mesh_parts: sco.meshes.iter().map(|_| Vec::new()).collect(),
                    scripts: None,
                    signal: None,
                };
                let mut lights = SceneryLightAssets::default();
                let mut assets = SceneryBuildAssets {
                    meshes: &mut meshes,
                    materials: &mut materials,
                    lights: &mut lights,
                };
                build_scenery_object(&mut commands, root, input, &mut assets);
            },
        )
        .expect("build");
    let cones = app
        .world_mut()
        .query::<&LightCone>()
        .iter(app.world())
        .cloned()
        .collect();
    let marked = app.world().get::<NoMapLighting>(root).is_some();
    Some((cones, marked))
}

pub(super) fn headlight_cones_of_model(
    path: &std::path::Path,
) -> Option<(usize, usize, Vec<LightCone>)> {
    let bytes = std::fs::read(path).ok()?;
    let model = ModelConfig::parse_bytes(&bytes).expect("model.cfg parses");
    let headlights = opts_bevy::vehicle_glow::detail_lights(&model)
        .filter(|l| opts_bevy::vehicle_glow::is_headlight(l))
        .count();
    let glows = opts_bevy::vehicle_glow::detail_lights(&model).count();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin));
    let root = app.world_mut().spawn(Transform::default()).id();
    {
        let mut commands = app.world_mut().commands();
        spawn_vehicle_glows(&mut commands, root, root, &model);
    }
    app.world_mut().flush();
    let cones = app
        .world_mut()
        .query::<&LightCone>()
        .iter(app.world())
        .cloned()
        .collect();
    Some((glows, headlights, cones))
}
