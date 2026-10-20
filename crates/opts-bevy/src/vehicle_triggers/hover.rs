//! The cockpit control under the cursor (M-0141): picking shared by the click system and the
//! hover feedback.

use bevy::ecs::system::SystemParam;
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::MouseTrigger;
use crate::camera::{CameraMode, CameraState};
use crate::vehicle_sim::PlayerVehicle;

/// Finds the hot-spot of the player's vehicle under the cursor in the cockpit and passenger views.
#[derive(SystemParam)]
pub struct HotSpotPicker<'w, 's> {
    state: Option<Res<'w, CameraState>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<crate::MainCamera>>,
    hot_spots: Query<'w, 's, &'static MouseTrigger>,
    vehicles: Query<'w, 's, (), With<PlayerVehicle>>,
    ray_cast: MeshRayCast<'w, 's>,
}

impl HotSpotPicker<'_, '_> {
    /// The nearest hot-spot under the cursor, `None` outside the interior views or when the
    /// cursor is not over one.
    pub fn under_cursor(&mut self) -> Option<MouseTrigger> {
        if !self
            .state
            .as_ref()
            .is_some_and(|s| matches!(s.mode, CameraMode::Cockpit | CameraMode::Passenger))
        {
            return None;
        }
        let cursor = self.windows.single().ok()?.cursor_position()?;
        let ray = self
            .cameras
            .iter()
            .find(|(c, _)| c.is_active)
            .and_then(|(c, t)| c.viewport_to_world(t, cursor).ok())?;
        let hot_spots = &self.hot_spots;
        let vehicles = &self.vehicles;
        let filter = |e: Entity| hot_spots.get(e).is_ok_and(|h| vehicles.contains(h.owner));
        let settings = MeshRayCastSettings::default().with_filter(&filter);
        let (entity, _) = *self.ray_cast.cast_ray(ray, &settings).first()?;
        self.hot_spots.get(entity).ok().cloned()
    }
}

/// The trigger name of the control under the cursor, `None` when there is none.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct HoveredControl(pub Option<String>);

/// Keeps [`HoveredControl`] current (written only when it changes).
pub fn hover_system(mut picker: HotSpotPicker, mut hovered: ResMut<HoveredControl>) {
    let now = picker.under_cursor().map(|h| h.trigger);
    if hovered.0 != now {
        hovered.0 = now;
    }
}
