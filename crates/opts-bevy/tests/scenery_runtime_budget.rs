//! Frame-budget check of the scenery runtime (`docs/src/architecture/scenery-runtime.md`,
//! "Ticking and budget"): thousands of self-written traffic-light heads, each with a script, three
//! `[visible]` lamp meshes and three lights, driven by a light program.
//!
//! The bound is deliberately loose (shared CI machines); the measured value is printed with
//! `cargo test -p opts-bevy --release --test scenery_runtime_budget -- --nocapture`.

use std::time::Instant;

use bevy::prelude::*;
use opts_bevy::scenery_runtime::{
    build_scenery_object, SceneryBuildAssets, SceneryLightAssets, SceneryObjectInput,
    SceneryRuntimePlugin,
};
use opts_bevy::{advance_scenery, ModelPart, SceneryClock, SceneryRuntimeSettings, SceneryScripts};
use opts_core::formats::scenery::SceneryObject;
use opts_sim::script::ScriptEngine;
use opts_traffic::SignalProgram;

const HEADS: usize = 3000;

const SCO: &str = "\
[mesh]
housing.o3d
[mesh]
lamp_red.o3d
[visible]
red
1
[mesh]
lamp_yellow.o3d
[visible]
yellow
1
[mesh]
lamp_green.o3d
[visible]
green
1
";

/// Phase states: 0-2 red, 3-5 red and yellow, 6-8 green, 9-11 yellow (docs `sco.md`).
const SCRIPT: &str = "\
{frame}
    (L.L.TrafficLightPhase) 3 < (S.L.red)
    (L.L.TrafficLightPhase) 9 >= (S.L.yellow)
    (L.L.TrafficLightPhase) 6 >= (L.L.TrafficLightPhase) 9 < && (S.L.green)
{end}
";

#[test]
fn thousands_of_signal_heads_fit_the_frame_budget() {
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
    let (template, diags) = ScriptEngine::from_sources(&[("head.osc", SCRIPT)]);
    assert!(diags.is_empty(), "{diags:?}");
    let template = std::sync::Mutex::new(template);
    let program =
        SignalProgram::new(vec![(1, 30.0), (4, 2.0), (7, 40.0), (10, 3.0)], 0.0).expect("program");
    app.add_systems(
        Startup,
        move |mut commands: Commands,
              mut meshes: ResMut<Assets<Mesh>>,
              mut materials: ResMut<Assets<StandardMaterial>>| {
            let material = materials.add(StandardMaterial::default());
            let mut lights = SceneryLightAssets::default();
            for i in 0..HEADS {
                let mesh_parts = sco
                    .meshes
                    .iter()
                    .map(|_| {
                        vec![ModelPart {
                            mesh: meshes.add(Cuboid::new(0.2, 0.2, 0.2)),
                            material: material.clone(),
                            vertices: 24,
                            light_variable: None,
                            pivot: None,
                        }]
                    })
                    .collect();
                let root = commands
                    .spawn((
                        Transform::from_xyz((i % 100) as f32 * 10.0, 0.0, (i / 100) as f32 * 10.0),
                        Visibility::Inherited,
                    ))
                    .id();
                let input = SceneryObjectInput {
                    sco: &sco,
                    mesh_parts,
                    scripts: Some(SceneryScripts::new(
                        template.lock().expect("template").fork(),
                        &[],
                        &[],
                        Vec::new(),
                    )),
                    signal: Some(program.clone()),
                };
                let mut assets = SceneryBuildAssets {
                    meshes: &mut meshes,
                    materials: &mut materials,
                    lights: &mut lights,
                };
                build_scenery_object(&mut commands, root, input, &mut assets);
            }
        },
    );
    app.update();
    app.insert_resource(SceneryClock::at(2026, 6, 29, 12.0 * 3600.0));
    app.world_mut()
        .resource_mut::<SceneryRuntimeSettings>()
        .automatic = false;
    advance_scenery(app.world_mut(), 1.0 / 60.0); // runs {init}
    let steps = 60;
    let started = Instant::now();
    for _ in 0..steps {
        advance_scenery(app.world_mut(), 1.0 / 60.0);
    }
    let per_step = started.elapsed().as_secs_f64() / f64::from(steps);
    let per_head_us = per_step * 1e6 / HEADS as f64;
    println!(
        "{HEADS} heads: {:.3} ms per step, {per_head_us:.3} us per head",
        per_step * 1e3
    );
    assert!(
        per_step < 0.25,
        "a full step of {HEADS} heads took {:.1} ms",
        per_step * 1e3
    );
}
