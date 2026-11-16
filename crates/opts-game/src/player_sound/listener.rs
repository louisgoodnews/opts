//! Where the listener is: inside the vehicle's body box, or in the cabin by the camera mode.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_bevy::MainCamera;

/// Whether `listener` (world position) is inside the vehicle at `vehicle`: a box about the body
/// of a solo bus (1.7 m either side, up to 3.6 m high, 8 m either way; ❓ the model's own bounds
/// would be better).
#[must_use]
pub fn listener_inside(vehicle: &GlobalTransform, listener: Vec3) -> bool {
    inside_box(vehicle, listener)
}

/// Whether the listener is in the cabin: the driver's and the passengers' views are, the external
/// view is not (its camera hangs a few metres behind the vehicle, still within a body-sized box);
/// the walking and the free camera are where their position says.
#[must_use]
pub fn listener_in_cabin(
    mode: Option<opts_bevy::CameraMode>,
    vehicle: &GlobalTransform,
    listener: Vec3,
) -> bool {
    match mode {
        Some(opts_bevy::CameraMode::Cockpit | opts_bevy::CameraMode::Passenger) => true,
        Some(opts_bevy::CameraMode::External) => false,
        _ => inside_box(vehicle, listener),
    }
}

fn inside_box(vehicle: &GlobalTransform, listener: Vec3) -> bool {
    let p = vehicle.affine().inverse().transform_point3(listener);
    p.x.abs() < 1.7 && p.y > -0.3 && p.y < 3.6 && p.z.abs() < 8.0
}

/// The camera mode and the listener's place.
#[derive(SystemParam)]
pub struct PlayerListener<'w, 's> {
    pub(super) camera: Option<Res<'w, opts_bevy::CameraState>>,
    pub(super) listener: Query<'w, 's, &'static GlobalTransform, With<MainCamera>>,
}
