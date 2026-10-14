//! The joint variables the scripts read must not depend on the rendering frame rate.
//!
//! `articulation_<k>_alpha` is computed in `FixedUpdate`; `articulation_<k>_beta` and `_gamma`
//! come from `follow_ground`, an `Update` system whose filter uses the frame's `delta_secs`
//! (`docs/src/architecture/articulation-physics.md`, P3). The same drive at 30, 60 and 144 frames
//! per second must publish the same sequence at every fixed step. Before the pitch and roll moved
//! into the fixed step (`sample_consist_ground`, M-0053 Part 3) this test failed: beta differed by
//! up to 0.46 degrees (peak 29.7) and gamma by 0.47 (peak 10.7) between 60 and 144 fps.

use std::time::Duration;

use bevy::{prelude::*, time::TimeUpdateStrategy};
use opts_bevy::{
    vehicle_physics_step_system, ArticulatedChain, ArticulatedSegment, ArticulationPlugin,
    PlayerVehicle, VehicleController, VehicleScripts,
};
use opts_core::formats::map::MapTerrain;
use opts_game::in_game::omsi_world::{follow_ground, sample_consist_ground, GroundIndex};
use opts_sim::script::ScriptEngine;
use opts_sim::vehicle::inputs::GearPosition;

const FIXED_HZ: f64 = 64.0;
const STEPS: usize = 900;

#[derive(Resource, Default)]
struct Log(Vec<[f32; 3]>);

#[derive(Resource)]
struct Root(Entity);

/// Rolling terrain (ridges every ~35 m north-south, every ~40 m east-west): the vehicle pitches
/// and rolls while it drives.
/// Terrain with ridges (`rolling`) or perfectly flat.
fn terrain(rolling: bool) -> MapTerrain {
    let dim = 61usize;
    let mut heights = vec![0.0; dim * dim];
    for j in 0..dim {
        let h = if rolling {
            2.5 * (j as f32 * 0.9).sin()
        } else {
            0.0
        };
        for i in 0..dim {
            heights[j * dim + i] = h + if rolling {
                1.2 * (i as f32 * 0.8).sin()
            } else {
                0.0
            };
        }
    }
    MapTerrain {
        grid_cells: 60,
        heights,
    }
}

fn record(root: Res<Root>, scripts: Query<&VehicleScripts>, mut log: ResMut<Log>) {
    let engine = scripts.get(root.0).unwrap().engine.lock().unwrap();
    log.0.push([
        engine.get("articulation_0_alpha"),
        engine.get("articulation_0_beta"),
        engine.get("articulation_0_gamma"),
    ]);
}

/// Drives for `STEPS` fixed steps with one rendering frame every `frame` seconds.
fn run(frame: f64, rolling: bool) -> Vec<[f32; 3]> {
    let mut app = App::new();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        frame,
    )));
    app.add_plugins(MinimalPlugins)
        .add_plugins(TransformPlugin)
        .add_plugins(ArticulationPlugin)
        .insert_resource(Time::<Fixed>::from_hz(FIXED_HZ))
        .init_resource::<Log>()
        .add_systems(FixedUpdate, vehicle_physics_step_system)
        .add_systems(FixedLast, record.run_if(|l: Res<Log>| l.0.len() < STEPS))
        .add_systems(
            FixedUpdate,
            sample_consist_ground
                .after(vehicle_physics_step_system)
                .before(opts_bevy::articulation_system),
        )
        .add_systems(Update, follow_ground);
    let mut ground = GroundIndex::default();
    ground.insert_tile((0, 0), (0.0, 0.0), Some(terrain(rolling)), &[]);
    app.insert_resource(ground);

    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.sim.chassis.position = Vec2::new(150.0, 10.0);
    controller.inputs_mut().set_parking_brake(false);
    controller.inputs_mut().set_gear(GearPosition::Drive);
    controller.inputs_mut().set_throttle(0.6);
    controller.inputs_mut().set_steering(0.15);
    let (engine, _) = ScriptEngine::from_sources(&[("t.osc", "{frame}\n{end}")]);
    let mut seg = ArticulatedSegment::new(Vec2::new(0.0, -5.0), Vec2::new(0.0, 1.0), -5.0, 52.5);
    seg.set_wheels(2, [0.5]);
    let segment = app.world_mut().spawn((seg, Transform::default())).id();
    let root = app
        .world_mut()
        .spawn((
            controller,
            Transform::default(),
            PlayerVehicle,
            ArticulatedChain {
                segments: vec![segment],
            },
            VehicleScripts::new(engine),
        ))
        .id();
    app.insert_resource(Root(root));
    let mut guard = 0;
    while app.world().resource::<Log>().0.len() < STEPS && guard < 100_000 {
        app.update();
        guard += 1;
    }
    app.world().resource::<Log>().0.clone()
}

#[test]
fn flat_ground_keeps_pitch_and_roll_at_zero() {
    for log in [run(1.0 / 60.0, false), run(1.0 / 30.0, false)] {
        for [_, beta, gamma] in log {
            assert!(beta.abs() < 0.01 && gamma.abs() < 0.01, "{beta} {gamma}");
        }
    }
}

#[test]
fn the_joint_variables_do_not_depend_on_the_frame_rate() {
    let a = run(1.0 / 60.0, true);
    for frame in [1.0 / 144.0, 1.0 / 30.0] {
        let b = run(frame, true);
        compare(&a, &b);
    }
}

fn compare(a: &[[f32; 3]], b: &[[f32; 3]]) {
    assert_eq!(a.len(), STEPS);
    assert_eq!(b.len(), STEPS);
    let worst = |k: usize| {
        a.iter()
            .zip(b)
            .map(|(x, y)| (x[k] - y[k]).abs())
            .fold(0.0_f32, f32::max)
    };
    let peak = |k: usize| a.iter().map(|x| x[k].abs()).fold(0.0_f32, f32::max);
    eprintln!(
        "alpha: worst difference {:.6}° (peak {:.2}°); beta: {:.6}° (peak {:.2}°); gamma: {:.6}° (peak {:.2}°)",
        worst(0),
        peak(0),
        worst(1),
        peak(1),
        worst(2),
        peak(2)
    );
    assert!(
        peak(0) > 0.5 && peak(1) > 0.5 && peak(2) > 0.1,
        "the drive must move all three joint angles: {:?}",
        [peak(0), peak(1), peak(2)]
    );
    for k in 0..3 {
        assert_eq!(worst(k), 0.0, "variable {k} differs between frame rates");
    }
}
