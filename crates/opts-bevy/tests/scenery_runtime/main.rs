//! Headless test of the scenery runtime (`docs/src/architecture/scenery-runtime.md`): a
//! self-written scenery object with LOD groups, a `[visible]` mesh, a `[newanim]` mesh and a light,
//! driven by a self-written script over simulated time.

use std::sync::Mutex;
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use opts_bevy::scenery_runtime::{
    build_scenery_object, SceneryBuildAssets, SceneryLight, SceneryLightAssets, SceneryLod,
    SceneryObjectInput, SceneryRuntimePlugin,
};
use opts_bevy::{
    advance_scenery, ModelPart, SceneryClock, SceneryObjectRuntime, SceneryRuntimeSettings,
    SceneryRuntimeStats, SceneryScripts,
};
use opts_core::formats::scenery::SceneryObject;
use opts_sim::script::ScriptEngine;

mod fixtures;
mod lamp;
mod lod;
mod motion;
use fixtures::*;

#[derive(Resource)]
struct Root(Entity);

fn app() -> App {
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
    .init_resource::<Assets<StandardMaterial>>()
    .add_plugins(SceneryRuntimePlugin);

    let sco = SceneryObject::from_text(SCO).expect("sco parses");
    let (engine, diags) = ScriptEngine::from_sources(&[("lamp.osc", SCRIPT)]);
    assert!(diags.is_empty(), "{diags:?}");
    let scripts = Mutex::new(Some(SceneryScripts::new(engine, &[], &[], Vec::new())));
    app.add_systems(
        Startup,
        move |mut commands: Commands,
              mut meshes: ResMut<Assets<Mesh>>,
              mut materials: ResMut<Assets<StandardMaterial>>| {
            let material = materials.add(StandardMaterial::default());
            let mesh_parts = sco
                .meshes
                .iter()
                .map(|_| {
                    vec![ModelPart {
                        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                        material: material.clone(),
                        vertices: 24,
                        light_variable: None,
                        alpha_variable: None,
                        image: None,
                        free_texture: None,
                        pivot: None,
                    }]
                })
                .collect();
            let root = commands
                .spawn((Transform::default(), Visibility::Inherited))
                .id();
            let input = SceneryObjectInput {
                sco: &sco,
                mesh_parts,
                scripts: scripts.lock().ok().and_then(|mut s| s.take()),
                signal: None,
            };
            let mut lights = SceneryLightAssets::default();
            let mut assets = SceneryBuildAssets {
                meshes: &mut meshes,
                materials: &mut materials,
                lights: &mut lights,
            };
            build_scenery_object(&mut commands, root, input, &mut assets);
            commands.insert_resource(Root(root));
            commands.spawn((
                Camera3d::default(),
                opts_bevy::MainCamera,
                Transform::from_xyz(0.0, 1.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ));
        },
    );
    app.update();
    app
}

fn runtime(app: &App) -> SceneryObjectRuntime {
    let root = app.world().resource::<Root>().0;
    app.world()
        .get::<SceneryObjectRuntime>(root)
        .expect("runtime state")
        .clone()
}

fn visibility(app: &App, e: Entity) -> Visibility {
    *app.world().get::<Visibility>(e).expect("visibility")
}

fn host_driven(app: &mut App, hour: f64) {
    app.insert_resource(SceneryClock::at(2026, 6, 29, hour * 3600.0));
    app.world_mut()
        .resource_mut::<SceneryRuntimeSettings>()
        .automatic = false;
}
