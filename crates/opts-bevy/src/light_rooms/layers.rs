//! The cabin layers: which render layers the cameras and lights use, and the cabin view state.

use super::*;

/// The render layers of the main camera: the world, plus the cabin lights when `cabin`.
#[must_use]
pub fn camera_layers(cabin: bool) -> RenderLayers {
    if cabin {
        RenderLayers::from_layers(&[WORLD_LAYER, CABIN_LAYER])
    } else {
        RenderLayers::layer(WORLD_LAYER)
    }
}

/// Whether the camera should see the cabin lights: always in an interior view; otherwise within
/// [`CABIN_ENTER_M`] of the player vehicle, and once it is in, until it is [`CABIN_LEAVE_M`]
/// away. `distance_m` is `None` without a player vehicle.
#[must_use]
pub fn cabin_layer_wanted(was_on: bool, interior_view: bool, distance_m: Option<f32>) -> bool {
    let Some(d) = distance_m else {
        return false;
    };
    if interior_view {
        return true;
    }
    d <= CABIN_ENTER_M || (was_on && d <= CABIN_LEAVE_M)
}

/// State the light systems share about the camera and the player vehicle.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CabinView {
    /// The camera is in an interior view of the player vehicle (cockpit or passenger).
    pub interior_view: bool,
    /// The main camera currently includes [`CABIN_LAYER`].
    pub cabin_layer: bool,
}

/// Whether the camera is in an interior view.
pub(super) fn interior_view(
    state: Option<&CameraState>,
    rigs: &Query<&VehicleCameraRig, With<crate::MainCamera>>,
) -> bool {
    state.is_some_and(|s| matches!(s.mode, CameraMode::Cockpit | CameraMode::Passenger))
        || rigs
            .iter()
            .any(|r| r.mode == VehicleCameraMode::InteriorCockpit)
}

/// Keeps the main camera's `RenderLayers` at `{0}` or `{0, 1}` and publishes [`CabinView`].
pub fn camera_light_layers_system(
    mut commands: Commands,
    state: Option<Res<CameraState>>,
    rigs: Query<&VehicleCameraRig, With<crate::MainCamera>>,
    mut view: ResMut<CabinView>,
    cameras: Query<(Entity, &GlobalTransform, Option<&RenderLayers>), With<crate::MainCamera>>,
    vehicle: Query<(&GlobalTransform, Option<&VehicleFootprint>), With<PlayerVehicle>>,
) {
    let interior = interior_view(state.as_deref(), &rigs);
    let Some((camera, cam_tf, layers)) = cameras.iter().next() else {
        return;
    };
    let distance = vehicle.iter().next().map(|(g, fp)| match fp {
        Some(fp) => fp.distance(g.affine().inverse().transform_point3(cam_tf.translation())),
        None => g.translation().distance(cam_tf.translation()),
    });
    let on = cabin_layer_wanted(view.cabin_layer, interior, distance);
    let next = CabinView {
        interior_view: interior,
        cabin_layer: on,
    };
    if *view != next {
        *view = next;
    }
    let want = camera_layers(on);
    if layers != Some(&want) {
        commands.entity(camera).insert(want);
    }
}
