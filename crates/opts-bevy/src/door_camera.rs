//! The door camera (M-0109): a picture-in-picture view from a door of the player's vehicle.
//!
//! * **Doors.** The door points are the pathpoints that the vehicle's passenger cabin names with
//!   `[entry]` and `[exit]` (indices into its `paths.cfg`, vehicle space: x right, y forward, z up),
//!   read once when the vehicle is built (`opts-game`, `vehicle/doors.rs`, which converts them to
//!   Bevy axes) and kept on the vehicle root as [`DoorPoints`]. The point lies on the floor at the door, so the camera sits above it, a little
//!   outside, and looks out and towards the rear ([`door_pose`]).
//! * **Which door.** Two sources, the manual one wins: the key **F4** cycles through the doors and
//!   back to none ([`DoorCamera::cycle`]); the automatic one shows the door whose leaf animates.
//!   Door state variables are named by each vehicle's scripts (`c2_tuer_1_v_pos`, ...), so the
//!   host cannot read them by name; it looks at the **mesh animations** instead: a mesh whose
//!   animation reads a variable that names a door ([`is_door_variable`]) with a shown value away
//!   from zero is an open door leaf, and the door point nearest to it ([`nearest_point`]) is the
//!   door ([`detect_open_doors`], five times a second, held for two seconds after the leaf rests).
//! * **Pictures.** Every open door (at most [`MAX_SLOTS`], in the order they opened) has its own
//!   [`DoorSlot`]: a `Camera3d` child of the vehicle, sitting at the door's upper left (as seen
//!   from outside) and looking into the doorway ([`door_pose`]), rendering into a 480 x 270 image
//!   at 30 Hz ([`RENDER_HZ`]); the HUD stacks the images from the top right downwards. A door that
//!   closes keeps its picture [`FADE_DELAY_S`], fades it out over [`FADE_S`] and then drops the
//!   slot, so the pictures below move up. Like the mirror cameras they render scene-linear (no
//!   tonemapping) and copy `Hdr` and fog from the [`MainCamera`].
//! * **Setting.** `camera.door_overlay` (off by default) sets [`DoorCamera::enabled`]; while it is
//!   off nothing exists: no camera, no image, no scan.

use bevy::camera::primitives::Aabb;
use bevy::camera::{Hdr, RenderTarget};
use bevy::pbr::DistanceFog;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::Tonemapping;

use crate::mirrors::MIRROR_ORDER;
use crate::vehicle_anim::MeshAnimation;
use crate::vehicle_sim::PlayerVehicle;
use crate::MainCamera;

mod systems;
pub use systems::*;
mod geometry;
mod points;
mod slot;

pub use geometry::*;
pub use points::*;
pub use slot::*;


/// Size of the picture (the HUD shows it as it is).
pub const IMAGE_SIZE: UVec2 = UVec2::new(480, 270);
/// How often the picture is rendered.
pub const RENDER_HZ: f64 = 30.0;
/// Height of the camera above the floor point of the door (m).
pub const EYE_HEIGHT_M: f32 = 2.1;
/// How far outside the door point the camera sits (m).
pub const EYE_OUT_M: f32 = 0.6;
/// How far to the upper left of the door (along the vehicle) the camera sits (m).
pub const EYE_ALONG_M: f32 = 0.9;
/// Seconds between two looks at the animated meshes.
pub const DETECT_INTERVAL_S: f64 = 0.2;
/// Seconds a door stays shown after its leaf stopped moving.
pub const HOLD_S: f64 = 2.0;
/// Pictures shown at once.
pub const MAX_SLOTS: usize = 4;
/// Seconds a closed door's picture stays before it fades.
pub const FADE_DELAY_S: f64 = 2.0;
/// Seconds the fade takes.
pub const FADE_S: f64 = 3.0;
/// A leaf is at most this far from its door point in the floor plane (m).
pub const LEAF_NEAR_M: f32 = 2.5;
/// A door animation value above this counts as open.
pub const OPEN_ABOVE: f32 = 0.05;


/// What the door camera shows and did.
#[derive(Resource, Debug, Clone, Default)]
pub struct DoorCamera {
    /// The setting `camera.door_overlay`.
    pub enabled: bool,
    /// The door chosen with the key (wins over the automatic one).
    pub manual: Option<usize>,
    /// The door whose leaf moves (or moved a moment ago).
    pub auto: Option<usize>,
    /// Real time until which `auto` is held.
    pub auto_until: f64,
    /// Every door whose leaf is open now (not held).
    pub open: Vec<usize>,
    /// The pictures, in the order the doors opened.
    pub slots: Vec<DoorSlot>,
    /// Real time of the last render request.
    pub last_render: f64,
    /// Pictures rendered since the session started.
    pub renders: u64,
}

impl DoorCamera {
    /// The door being shown first (`None`: nothing, and the cameras are off).
    #[must_use]
    pub fn shown(&self) -> Option<usize> {
        if self.enabled {
            self.manual.or(self.auto)
        } else {
            None
        }
    }

    /// The doors that should have a picture now: the manual one and the open ones.
    #[must_use]
    pub fn wanted(&self) -> Vec<usize> {
        if !self.enabled {
            return Vec::new();
        }
        let mut w: Vec<usize> = self.manual.into_iter().collect();
        for d in &self.open {
            if !w.contains(d) {
                w.push(*d);
            }
        }
        w
    }

    /// F4: the next door, after the last none again.
    pub fn cycle(&mut self, doors: usize) {
        self.manual = match self.manual {
            None if doors > 0 => Some(0),
            Some(i) if i + 1 < doors => Some(i + 1),
            _ => None,
        };
    }
}

/// Marks the door camera entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct DoorCameraMarker;

/// F4 cycles through the doors.
pub fn door_keys(
    keys: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<crate::action_keys::ActionKeys>>,
    vehicles: Query<&DoorPoints, With<PlayerVehicle>>,
    mut state: ResMut<DoorCamera>,
) {
    if !state.enabled
        || !crate::action_keys::resolve(&action_keys).just_pressed(&keys, "door_camera")
    {
        return;
    }
    let doors = vehicles.iter().next().map_or(0, |d| d.0.len());
    state.cycle(doors);
}

/// Registers the door camera.
pub struct DoorCameraPlugin;

impl Plugin for DoorCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DoorCamera>().add_systems(
            Update,
            (
                door_keys.run_if(crate::reporting::not_modal_active),
                detect_open_doors,
                drive_camera.run_if(resource_exists::<Assets<Image>>),
                sync_look,
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests;
