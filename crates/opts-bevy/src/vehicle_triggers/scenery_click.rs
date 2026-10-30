//! Clicks on placed scenery (M-0113): a mesh of a scenery object with `[mouseevent] <name>` (the
//! tongue of rail points answers `toggle_switch`) raises a [`SceneryClicked`] message on a left
//! click. The game decides what the trigger does, since it owns the state behind it (the points
//! position is shared with the trains, not only the object's own `Switch` variable).
//!
//! Picking reuses the cockpit's mesh ray cast ([`HotSpotPicker`]); the vehicle's own hot-spots
//! win, so a cockpit control is never shadowed by scenery behind it.

use bevy::ecs::system::SystemParam;
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::HotSpotPicker;

/// Scenery is clickable from this far (m): a switch tongue is operated by a driver standing near.
pub const REACH_M: f32 = 60.0;

/// A scenery mesh that raises a trigger when clicked (`[mouseevent]` of a `.sco` mesh).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct SceneryMouseTrigger {
    /// The object root (carries the placement and the scripts).
    pub owner: Entity,
    /// Trigger name.
    pub trigger: String,
}

/// A left click landed on a [`SceneryMouseTrigger`] mesh.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct SceneryClicked {
    /// The object root that was clicked.
    pub owner: Entity,
    /// The mesh's trigger name.
    pub trigger: String,
}

/// Finds the scenery hot-spot under the cursor.
#[derive(SystemParam)]
pub struct SceneryPicker<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<crate::MainCamera>>,
    spots: Query<'w, 's, &'static SceneryMouseTrigger>,
    ray_cast: MeshRayCast<'w, 's>,
}

impl SceneryPicker<'_, '_> {
    /// The nearest scenery hot-spot under the cursor within [`REACH_M`].
    pub fn under_cursor(&mut self) -> Option<SceneryMouseTrigger> {
        let cursor = self.windows.single().ok()?.cursor_position()?;
        let ray = self
            .cameras
            .iter()
            .find(|(c, _)| c.is_active)
            .and_then(|(c, t)| c.viewport_to_world(t, cursor).ok())?;
        let spots = &self.spots;
        let filter = |e: Entity| spots.contains(e);
        let settings = MeshRayCastSettings::default().with_filter(&filter);
        let (entity, hit) = self.ray_cast.cast_ray(ray, &settings).first()?;
        if hit.distance > REACH_M {
            return None;
        }
        self.spots.get(*entity).ok().cloned()
    }
}

/// Parameters of [`scenery_click_system`].
#[derive(SystemParam)]
pub struct SceneryClickParams<'w, 's> {
    mouse: Res<'w, ButtonInput<MouseButton>>,
    vehicle_spots: HotSpotPicker<'w, 's>,
    scenery: SceneryPicker<'w, 's>,
    driving: Option<Res<'w, crate::mouse_driving::MouseDriving>>,
    clicked: MessageWriter<'w, SceneryClicked>,
}

/// Turns a left click on a scenery hot-spot into a [`SceneryClicked`] message.
pub fn scenery_click_system(mut p: SceneryClickParams) {
    if !p.mouse.just_pressed(MouseButton::Left) || p.driving.is_some_and(|d| d.drives()) {
        return;
    }
    // A cockpit control under the cursor takes the click.
    if p.vehicle_spots.under_cursor().is_some() {
        return;
    }
    if let Some(spot) = p.scenery.under_cursor() {
        p.clicked.write(SceneryClicked {
            owner: spot.owner,
            trigger: spot.trigger,
        });
    }
}
