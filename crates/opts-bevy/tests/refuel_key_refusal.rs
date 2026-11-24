//! The refuel key F12 (M-0123): a refuel refused because the vehicle moves says so with a toast
//! and fills nothing; a stopped vehicle is filled without one.

use std::time::Duration;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use opts_bevy::vehicle_remote::RemoteControls;
use opts_bevy::vehicle_sim::vehicle_input_system;
use opts_bevy::{PlayerVehicle, VehicleController};
use opts_gui::components::toast::M3Toast;
use opts_sim::vehicle::fuel::{FuelKind, FuelMode};

/// A world with the player's bus at an empty diesel store, moving at `speed` m/s (forward).
fn world_with_bus(speed: f32) -> World {
    let mut world = World::new();
    world.init_resource::<ButtonInput<KeyCode>>();
    world.init_resource::<Time>();
    world.init_resource::<RemoteControls>();
    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller
        .sim
        .ensure_fuel(FuelKind::Diesel, 250.0, FuelMode::Realistic);
    controller.sim.fuel.as_mut().expect("fuel").set_level(0.0);
    controller.sim.chassis.local_velocity.y = speed;
    world.spawn((controller, PlayerVehicle));
    world
}

/// Presses F12 and runs the keyboard system once (the press is read in that frame).
fn press_refuel(world: &mut World) {
    world
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(16));
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F12);
    world
        .run_system_once(vehicle_input_system)
        .expect("the keyboard system runs");
}

fn level(world: &mut World) -> f32 {
    let mut q = world.query_filtered::<&VehicleController, With<PlayerVehicle>>();
    q.iter(world)
        .next()
        .expect("player")
        .sim
        .fuel_level()
        .expect("fuel")
}

fn toasts(world: &mut World) -> usize {
    world.query::<&M3Toast>().iter(world).count()
}

#[test]
fn a_refused_refuel_key_press_shows_a_toast_and_fills_nothing() {
    for speed in [0.51_f32, -0.51, 5.0] {
        let mut world = world_with_bus(speed);
        press_refuel(&mut world);
        assert_eq!(level(&mut world), 0.0, "{speed} m/s: store unchanged");
        assert_eq!(toasts(&mut world), 1, "{speed} m/s: the refusal is shown");
    }
}

#[test]
fn a_stopped_refuel_key_press_fills_without_a_toast() {
    for speed in [0.0_f32, 0.49, -0.49] {
        let mut world = world_with_bus(speed);
        press_refuel(&mut world);
        assert_eq!(level(&mut world), 1.0, "{speed} m/s: store filled");
        assert_eq!(toasts(&mut world), 0, "{speed} m/s: no refusal toast");
    }
}
