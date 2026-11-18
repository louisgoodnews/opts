//! The headless rig: a bus in a `World`, stepped and rendered by hand.

use std::time::Duration;

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use opts_bevy::{
    vehicle_physics_step_system, vehicle_transform_sync_system, PlayerVehicle, RenderPose,
    VehicleController,
};
use opts_core::formats::map::MapTerrain;
use opts_game::in_game::omsi_world::{follow_ground, GroundIndex};
use opts_sim::vehicle::inputs::GearPosition;

pub const FIXED_HZ: f64 = 64.0;

/// Terrain covering the test area (a 300 m square in 5 m cells): flat, and with a ridge of
/// `ridge_m` from the 10th row of cells (y = 50 m) on, a step the bus drives up.
fn ground_with_ridge(ridge_m: f32) -> GroundIndex {
    let dim = 61usize;
    let heights = (0..dim * dim)
        .map(|k| if k / dim >= 10 { ridge_m } else { 0.0 })
        .collect();
    let terrain = MapTerrain {
        grid_cells: 60,
        heights,
    };
    let mut ground = GroundIndex::default();
    ground.insert_tile(
        (0, 0),
        (0.0, 0.0),
        Some(opts_bevy::TileTerrain::new(terrain, 5.0)),
        &[],
    );
    ground
}

/// A world with one player bus on terrain with a ridge of `ridge_m`, accelerating in a straight
/// line.
fn world_with_bus(ridge_m: f32) -> World {
    let mut world = World::new();
    world.insert_resource(Time::<Fixed>::from_hz(FIXED_HZ));
    world.init_resource::<Time>();
    world.insert_resource(ground_with_ridge(ridge_m));

    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.sim.chassis.position = Vec2::new(150.0, 10.0);
    controller.inputs_mut().set_parking_brake(false);
    controller.inputs_mut().set_gear(GearPosition::Drive);
    controller.inputs_mut().set_throttle(0.6);
    world.spawn((controller, Transform::default(), PlayerVehicle));
    world
}

fn timestep() -> Duration {
    Duration::from_secs_f64(1.0 / FIXED_HZ)
}

/// One physics step of the bus, as `FixedUpdate` runs it.
fn physics_step(world: &mut World) {
    world.resource_mut::<Time>().advance_by(timestep());
    world
        .run_system_once(vehicle_physics_step_system)
        .expect("physics step runs");
}

/// The bus's controller.
pub fn controller(world: &mut World) -> &VehicleController {
    world
        .query::<&VehicleController>()
        .single(world)
        .expect("one vehicle")
}

/// Runs one physics step with the given inputs and returns the pose before it and after it.
pub fn measured_step(world: &mut World, steering: f32, throttle: f32) -> (RenderPose, RenderPose) {
    {
        let mut q = world.query::<&mut VehicleController>();
        let mut c = q.single_mut(world).expect("one vehicle");
        c.inputs_mut().set_steering(steering);
        c.inputs_mut().set_throttle(throttle);
    }
    physics_step(world);
    let c = controller(world);
    (
        c.render_prev.expect("the step recorded the pose before it"),
        c.pose(),
    )
}

/// Renders the bus at an overstep fraction: the transform sync, then the ground follower, as in
/// `Update`. Returns the root transform.
pub fn render_at(world: &mut World, alpha: f32) -> Transform {
    let mut fixed = Time::<Fixed>::from_hz(FIXED_HZ);
    fixed.accumulate_overstep(timestep().mul_f32(alpha));
    *world.resource_mut::<Time<Fixed>>() = fixed;
    world
        .run_system_once(vehicle_transform_sync_system)
        .expect("sync runs");
    world.run_system_once(follow_ground).expect("follow runs");
    *world
        .query::<&Transform>()
        .single(world)
        .expect("one transform")
}

/// Brings the bus to a steady speed so the next steps have a real yaw rate and body motion.
fn warm_up(world: &mut World) {
    for _ in 0..128 {
        physics_step(world);
    }
}

/// A bus on flat ground that turns through one physics step, with the step's pose before and
/// after.
pub fn turning_bus() -> (World, RenderPose, RenderPose) {
    let mut world = world_with_bus(0.0);
    warm_up(&mut world);
    let (prev, curr) = measured_step(&mut world, 0.5, 1.0);
    (world, prev, curr)
}

/// A bus driving straight up a ridge: the first physics step (within 600) that moves both the
/// heave and the pitch of the body, with the step's pose before and after.
pub fn crossing_bus() -> (World, RenderPose, RenderPose) {
    let mut world = world_with_bus(0.12);
    warm_up(&mut world);
    for _ in 0..600 {
        let (prev, curr) = measured_step(&mut world, 0.0, 1.0);
        if (curr.heave_m - prev.heave_m).abs() > 1e-4
            && (curr.pitch_rad - prev.pitch_rad).abs() > 1e-4
        {
            return (world, prev, curr);
        }
    }
    panic!("the bus must cross the ridge within 600 steps");
}
