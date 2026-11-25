//! Bevy plugin wiring the vehicle physics, input, transform and camera systems.

use bevy::prelude::*;

use super::camera::vehicle_camera_system;
use super::input::vehicle_input_system;
use super::sync::{
    vehicle_physics_step_system, vehicle_transform_sync_system, wheel_transform_sync_system,
};

/// Bevy plugin bundling vehicle physics simulation, input systems, and transform sync.
#[derive(Default)]
pub struct OptsVehiclePlugin;

impl Plugin for OptsVehiclePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::vehicle_remote::RemoteControls>()
            .init_resource::<crate::mouse_driving::MouseDriving>()
            .init_resource::<crate::gamepad_input::PadBindings>()
            .add_systems(FixedUpdate, vehicle_physics_step_system)
            .add_systems(
                Update,
                (
                    vehicle_input_system
                        .run_if(crate::camera::vehicle_accepts_input)
                        .run_if(crate::reporting::not_modal_active),
                    // The mouse owns the pedals while its mode drives (after the keyboard).
                    (
                        crate::mouse_driving::mouse_driving_buttons,
                        crate::mouse_driving::mouse_driving_axes,
                    )
                        .chain()
                        .after(vehicle_input_system)
                        .run_if(crate::camera::vehicle_accepts_input)
                        .run_if(crate::reporting::not_modal_active),
                    // The pad axes and buttons follow the mouse: an axis past its dead zone wins.
                    (
                        crate::gamepad_input::pad_driving_system,
                        crate::gamepad_input::pad_button_system,
                    )
                        .after(vehicle_input_system)
                        .after(crate::mouse_driving::mouse_driving_axes)
                        .run_if(crate::camera::vehicle_accepts_input)
                        .run_if(crate::reporting::not_modal_active),
                    crate::mouse_driving::mouse_driving_gate_closed
                        .run_if(crate::vehicle_remote::vehicle_input_closed),
                    crate::vehicle_remote::flush_remote_when_gated
                        .run_if(crate::vehicle_remote::vehicle_input_closed),
                    crate::vehicle_remote::release_trigger_system,
                    crate::rail_controls::direction_system.after(vehicle_input_system),
                    crate::cab_switch::cab_handover_system,
                    crate::cab_switch::cab_trace_system,
                    crate::cab_switch::cab_switch_system
                        .before(crate::rail_controls::direction_system)
                        .run_if(crate::reporting::not_modal_active),
                    crate::vehicle_remote::odometer_system,
                    vehicle_transform_sync_system,
                    wheel_transform_sync_system,
                ),
            )
            .add_systems(
                PostUpdate,
                vehicle_camera_system.run_if(crate::camera::rig_drives_camera),
            );
    }
}
