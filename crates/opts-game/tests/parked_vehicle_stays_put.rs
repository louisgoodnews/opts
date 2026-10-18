//! A vehicle that stands on the map with nobody at the pedals stays where it was put.
//!
//! UAT (Berlin-Spandau): "spawned busses move away from the spawn point after a few seconds, in
//! any direction, sometimes right at the spawn". The causes were found with a running game
//! (docs/src/architecture/ai-traffic.md, "The player's vehicles are obstacles"):
//!
//! * AI vehicles knew nothing of the player's bus and drove through it; the collision response
//!   shoved the bus along in front of them (`opts-traffic` test
//!   `an_agent_stops_for_an_obstacle_in_its_lane_and_passes_one_beside_it`);
//! * a spawn point inside the collision box of a scenery object pushed the bus out at once.
//!
//! This test runs the physics and the player's collision pass for 60 s of simulated time on flat
//! and on sloped terrain, with and without a collision box around the spawn point.

use std::time::Duration;

use bevy::{prelude::*, time::TimeUpdateStrategy};
use opts_bevy::{PlayerVehicle, VehicleController};
use opts_core::formats::map::MapTerrain;
use opts_game::in_game::omsi_world::{
    follow_ground, player_collisions, CollisionIndex, GroundIndex, PlayerBox,
};
use opts_sim::collision::Obb;

const FIXED_HZ: f64 = 64.0;
const SECONDS: f64 = 60.0;

/// Terrain 1 m of height per 5 m cell to the east (a 11 % slope) or flat.
fn terrain(slope: bool) -> MapTerrain {
    let dim = 61usize;
    let mut heights = vec![0.0; dim * dim];
    for j in 0..dim {
        for i in 0..dim {
            heights[j * dim + i] = if slope { i as f32 * 0.55 } else { 0.0 };
        }
    }
    MapTerrain {
        grid_cells: 60,
        heights,
    }
}

/// Runs a parked bus for 60 s; returns how far it moved and turned.
fn drift(slope: bool, inside_box: bool, parking_brake: bool) -> (f32, f32) {
    let mut app = App::new();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    app.add_plugins(MinimalPlugins)
        .add_plugins(TransformPlugin)
        .insert_resource(Time::<Fixed>::from_hz(FIXED_HZ))
        .init_resource::<CollisionIndex>()
        .add_systems(
            FixedUpdate,
            (opts_bevy::vehicle_physics_step_system, player_collisions).chain(),
        )
        .add_systems(Update, follow_ground);
    let mut ground = GroundIndex::default();
    ground.insert_tile(
        (0, 0),
        (0.0, 0.0),
        Some(opts_bevy::TileTerrain::new(terrain(slope), 5.0)),
        &[],
    );
    app.insert_resource(ground);

    let start = Vec2::new(150.0, 150.0);
    if inside_box {
        // A scenery object's collision box that contains the front part of the bus.
        app.world_mut().resource_mut::<CollisionIndex>().insert_box(
            (0, 0),
            Obb::new(
                start + Vec2::new(0.0, 6.0),
                Vec2::new(3.0, 3.0),
                0.3,
                -1.0,
                4.0,
            ),
        );
    }
    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.sim.chassis.position = start;
    controller.sim.chassis.heading_rad = 0.5;
    controller.inputs_mut().set_parking_brake(parking_brake);
    app.world_mut().spawn((
        controller,
        Transform::default(),
        PlayerVehicle,
        PlayerBox(opts_game::in_game::omsi_world::BodyBox {
            half: Vec2::new(1.25, 6.0),
            offset: Vec2::ZERO,
            height: 3.0,
        }),
    ));
    let frames = (SECONDS * 60.0) as usize;
    for _ in 0..frames {
        app.update();
    }
    let mut q = app.world_mut().query::<&VehicleController>();
    let c = q.single(app.world()).unwrap();
    let ch = &c.sim.chassis;
    ((ch.position - start).length(), (ch.heading_rad - 0.5).abs())
}

#[test]
fn a_parked_vehicle_stays_within_a_few_centimetres_for_a_minute() {
    for slope in [false, true] {
        let (moved, turned) = drift(slope, false, true);
        assert!(moved < 0.03, "slope {slope}: moved {moved} m");
        assert!(turned < 0.005, "slope {slope}: turned {turned} rad");
    }
}

#[test]
fn a_vehicle_in_neutral_without_the_parking_brake_does_not_roll_on_flat_ground() {
    let (moved, _) = drift(false, false, false);
    assert!(moved < 0.03, "moved {moved} m");
}

#[test]
fn a_vehicle_spawned_inside_a_collision_box_is_not_pushed_out() {
    for slope in [false, true] {
        let (moved, turned) = drift(slope, true, true);
        assert!(moved < 0.03, "slope {slope}: moved {moved} m");
        assert!(turned < 0.005, "slope {slope}: turned {turned} rad");
    }
}
