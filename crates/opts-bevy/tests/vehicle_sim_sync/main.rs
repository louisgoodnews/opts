//! Integration tests for the Bevy vehicle physics simulation bridge.

use std::time::Duration;

use bevy::{input::InputPlugin, prelude::*, time::TimeUpdateStrategy};
use opts_bevy::{
    OptsVehiclePlugin, PlayerVehicle, VehicleCameraMode, VehicleCameraRig, VehicleController,
    WheelSide, WheelVisual,
};
use opts_sim::vehicle::inputs::GearPosition;

fn create_test_app() -> App {
    let mut app = App::new();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
        1.0 / 60.0,
    )));
    app.add_plugins(MinimalPlugins)
        .add_plugins(TransformPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(OptsVehiclePlugin);
    app
}

mod camera_rig;
mod fixed_step;
mod transform_sync;
mod wheels;
