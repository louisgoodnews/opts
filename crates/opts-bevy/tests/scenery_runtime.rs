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

const SCO: &str = "\
[mesh]
pole.o3d

[LOD]
0.2

[mesh]
lamp_glass.o3d

[visible]
lamp_on
1

[mesh]
hand.o3d

[newanim]
origin_trans
0
0
2
anim_rot
hand_deg
1

[light_enh_2]
0
-0.2
2
0
-1
0
0
0
1
1
0
255
220
150
0.4
180
180
lamp_on
1
0
0
0
0

[LOD]
0

[mesh]
stub.o3d
";

/// Lit between 18:00 and 06:00; the hand turns 10 degrees per simulated second.
const SCRIPT: &str = "\
{frame}
    (L.S.Time) 64800 >= (L.S.Time) 21600 < || (S.L.lamp_on)
    (L.S.Timegap) 10 * (L.L.hand_deg) + (S.L.hand_deg)
{end}
";

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
                        image: None,
                        pivot: None,
                        uv_unreliable: 0,
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

#[test]
fn the_script_switches_the_lamp_by_time_of_day() {
    let mut app = app();
    host_driven(&mut app, 12.0);
    advance_scenery(app.world_mut(), 1.0 / 60.0);
    let rt = runtime(&app);
    let (glass, light) = (rt.meshes[0], rt.lights[0]);
    assert_eq!(visibility(&app, glass), Visibility::Hidden);
    assert_eq!(app.world().get::<SceneryLight>(light).unwrap().level, 0.0);

    host_driven(&mut app, 23.0);
    advance_scenery(app.world_mut(), 1.0 / 60.0);
    assert_eq!(visibility(&app, glass), Visibility::Inherited);
    assert_eq!(app.world().get::<SceneryLight>(light).unwrap().level, 1.0);
    // The sprite of a lit, omnidirectional light shows after the frame's light update.
    app.update();
    assert_eq!(visibility(&app, light), Visibility::Inherited);
}

#[test]
fn the_animated_mesh_moves_with_simulated_time() {
    let mut app = app();
    host_driven(&mut app, 12.0);
    let hand = runtime(&app).meshes[1];
    for _ in 0..60 {
        advance_scenery(app.world_mut(), 1.0 / 60.0);
    }
    let after_1s = *app.world().get::<Transform>(hand).unwrap();
    for _ in 0..60 {
        advance_scenery(app.world_mut(), 1.0 / 60.0);
    }
    let after_2s = *app.world().get::<Transform>(hand).unwrap();
    let angle = |t: &Transform| t.rotation.angle_between(Quat::IDENTITY).to_degrees();
    assert!((angle(&after_1s) - 10.0).abs() < 0.5, "{after_1s:?}");
    assert!((angle(&after_2s) - 20.0).abs() < 0.5, "{after_2s:?}");
    let root = app.world().resource::<Root>().0;
    let scripts = app.world().get::<SceneryScripts>(root).unwrap();
    assert_eq!(scripts.ticks, 120);
    let deg = scripts.with_engine(|e| e.get("hand_deg")).unwrap();
    assert!((deg - 20.0).abs() < 1e-3, "{deg}");
    let clock = app.world().resource::<SceneryClock>().seconds_of_day;
    assert!((clock - (12.0 * 3600.0 + 2.0)).abs() < 1e-4, "{clock}");
}

#[test]
fn the_runtime_ticks_by_itself_from_bevy_time() {
    let mut app = app();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        100,
    )));
    for _ in 0..11 {
        app.update();
    }
    let root = app.world().resource::<Root>().0;
    let deg = app
        .world()
        .get::<SceneryScripts>(root)
        .unwrap()
        .with_engine(|e| e.get("hand_deg"))
        .unwrap();
    assert!(deg > 5.0, "the near object ticks every frame: {deg}");
    let stats = app.world().resource::<SceneryRuntimeStats>();
    assert_eq!((stats.objects, stats.scripted), (1, 1));
}

#[test]
fn lod_shows_the_band_of_the_current_screen_size() {
    let mut app = app();
    let groups = {
        let root = app.world().resource::<Root>().0;
        app.world().get::<SceneryLod>(root).unwrap().groups.clone()
    };
    let (fine, coarse) = (groups[0], groups[1]);
    assert_eq!((fine.1, coarse.1), (0.2, 0.0));
    app.update();
    assert_eq!(visibility(&app, fine.0), Visibility::Inherited);
    assert_eq!(visibility(&app, coarse.0), Visibility::Hidden);

    // Far away the object is small: the stand-in shows instead.
    let mut q = app
        .world_mut()
        .query_filtered::<&mut Transform, With<Camera3d>>();
    for mut t in q.iter_mut(app.world_mut()) {
        *t = Transform::from_xyz(0.0, 1.0, 400.0).looking_at(Vec3::ZERO, Vec3::Y);
    }
    app.update();
    app.update();
    assert_eq!(visibility(&app, fine.0), Visibility::Hidden);
    assert_eq!(visibility(&app, coarse.0), Visibility::Inherited);
}
