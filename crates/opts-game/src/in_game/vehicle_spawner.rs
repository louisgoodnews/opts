//! Tracking camera rig for the player vehicle. The vehicle itself is loaded from its `.bus`
//! and model config by [`crate::in_game::omsi_world::load_player_vehicle`].

use bevy::prelude::*;
use opts_bevy::{MainCamera, VehicleCameraMode, VehicleCameraRig};

use crate::in_game::omsi_world::CameraSetup;
use crate::in_game::types::{InGameCamera, InGameEntity};

/// Spawns the 3D tracking camera with a [`VehicleCameraRig`] sized from the loaded vehicle.
pub fn spawn_camera_rig(commands: &mut Commands, setup: &CameraSetup) {
    // The external view starts far enough back to see the whole vehicle (about its length plus
    // a margin), and the reset key returns to that distance.
    let zoom = setup.chase_distance.clamp(
        opts_bevy::camera::mode::vehicle_views::EXTERNAL_DEFAULT_ZOOM_M,
        40.0,
    );
    commands.queue(move |world: &mut World| {
        if let Some(mut state) = world.get_resource_mut::<opts_bevy::CameraState>() {
            state.external_zoom = zoom;
            state.external_home_zoom = zoom;
        }
    });
    commands.spawn((
        (
            Camera3d::default(),
            MainCamera,
            bevy::ui::IsDefaultUiCamera,
            setup.start,
        ),
        VehicleCameraRig {
            mode: VehicleCameraMode::ExteriorChase,
            chase_distance: setup.chase_distance,
            chase_height: setup.chase_height,
            cockpit_offset: setup.cockpit_offset,
            damping: 8.0,
        },
        InGameEntity,
        InGameCamera,
        Name::new("Simulation_3D_Tracking_Camera"),
    ));
}

/// Spawns a plain 3D camera for free-roam inspection when there is no vehicle to track.
pub fn spawn_free_camera(commands: &mut Commands, transform: Transform) {
    commands.spawn((
        (
            Camera3d::default(),
            MainCamera,
            bevy::ui::IsDefaultUiCamera,
            transform,
        ),
        InGameEntity,
        InGameCamera,
        Name::new("Free_Roam_Camera"),
    ));
}
