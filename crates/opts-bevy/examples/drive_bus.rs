//! Example: Interactive Bus Driving & Simulation Bridge in Bevy (`opts-bevy`).
//!
//! Demonstrates:
//! 1. Initializing the [`OptsVehiclePlugin`] and deterministic physics simulation bridge.
//! 2. Spawning a 12-meter commercial transit bus model with Ackermann-steered front wheels.
//! 3. Real-time driver keyboard controls (WASD/Arrows, Space handbrake, E/N/R gears).
//! 4. Exterior chase and interior cockpit camera tracking with F1 / F2 toggle.
//! 5. On-screen telemetry HUD showing road speed, engine RPM, gear, and controls.
//!
//! Run with:
//! ```bash
//! cargo run --example drive_bus
//! ```

use bevy::prelude::*;
use opts_bevy::{
    OptsAssetPlugin, PlayerVehicle, VehicleCameraMode, VehicleCameraRig, VehicleController,
    WheelSide, WheelVisual,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "OPTS — Interactive Bus Driving Simulation".into(),
                resolution: bevy::window::WindowResolution::new(1920, 1080),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(OptsAssetPlugin)
        .add_systems(Startup, (setup_world, setup_vehicle, setup_hud))
        .add_systems(Update, (update_telemetry_hud, handle_exit))
        .run();
}

#[path = "drive_bus/hud.rs"]
mod hud;
#[path = "drive_bus/vehicle.rs"]
mod vehicle;
#[path = "drive_bus/world.rs"]
mod world;
use hud::*;
use vehicle::*;
use world::*;

/// Closes the application gracefully when Escape is pressed.
fn handle_exit(keyboard: Res<ButtonInput<KeyCode>>, mut exit_events: MessageWriter<AppExit>) {
    if keyboard.just_pressed(KeyCode::Escape) {
        exit_events.write(AppExit::Success);
    }
}
