//! Vehicle physics simulation bridge and ECS integration for Bevy.
//!
//! Connects the authoritative [`opts_sim::vehicle::VehicleSimulation`] domain model from `opts-sim`
//! to Bevy's ECS architecture. Coordinates deterministic fixed-step physics execution
//! in [`bevy::app::FixedUpdate`], smooth player input ingestion, chassis coordinate projection
//! into 3D transforms, dynamic wheel steering and rolling mesh animations, and
//! chase/cockpit camera tracking.

mod camera;
mod controller;
mod input;
mod plugin;
mod sync;

pub use camera::{vehicle_camera_system, VehicleCameraMode, VehicleCameraRig};
pub use controller::{
    PlayerVehicle, RenderPose, VehicleController, VehicleInputConfig, WheelSide, WheelVisual,
};
pub use input::vehicle_input_system;
pub use plugin::OptsVehiclePlugin;
pub use sync::{
    vehicle_physics_step_system, vehicle_transform_sync_system, wheel_transform_sync_system,
};
