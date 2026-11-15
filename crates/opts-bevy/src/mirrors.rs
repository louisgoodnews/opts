//! Rear-view mirror cameras of the player's vehicle (`docs/src/architecture/reflections-research.md`
//! section 7 step A, `lighting-system.md` section 9, `camera-system.md`).
//!
//! * **Identification.** A mirror pane is a mesh of the [`PlayerVehicle`] whose material the model
//!   loader registered as [`SurfaceKind::Mirror`]: the texture `reflexion<N>.bmp`, OMSI's
//!   convention for the pane of mirror `N` (the housing of the same unit has another texture, so
//!   the pane is a separate part with only pane faces). AI and parked vehicles are never looked
//!   at. A scan every [`SCAN_INTERVAL_S`] real seconds finds new panes (the vehicle's parts spawn
//!   over several frames). Panes of one `reflexion<N>.bmp` number form a **group** ([`GroupKey`]: the
//!   main and the wide-angle pane of a side mirror have different pooled materials, and the
//!   alternative mirror variants of a model, only one of which is visible, are more panes of the
//!   same number); a pane without a number groups by its material. A group gets one camera, at
//!   most [`MAX_CAMERAS`] groups. (Grouping by material alone once let the hidden variants use up
//!   the cameras, and the visible left mirror kept the static picture.)
//! * **One camera per group**, a `Camera3d` rendering into a [`MirrorSettings::size`] image
//!   (8-bit; the camera carries `Hdr` like the main camera). The pose is the viewer-ray
//!   reflection of [`mirror_camera_pose`]: the camera sits just in front of the visible members'
//!   centre and looks along the main camera's view ray reflected at their mean normal. The
//!   vertical field of view is the angle the pane subtends from the viewer ([`mirror_fov`]); the
//!   projection aspect is the pane's own, so the picture is not distorted when it is stretched
//!   over the pane's UV rectangle ([`pane_uv_transform`]). The picture replaces the base-colour
//!   texture of a **private copy** of the material (the pooled material is shared with every other
//!   vehicle of the same type); the original comes back while `mirror_count` is 0 and no
//!   virtual mirror streams, or when the vehicle is no longer the player's.
//! * **Budget** (`mirrors/schedule.rs`). Every group with a pane **in the main camera's view**
//!   ([`mirror_in_view`], at least [`MIRROR_MIN_ANGLE`] across) shows a live picture.
//!   [`LightQuality::mirror_count`] pictures render every `mirror_every` frames
//!   ([`renders_in_frame`]: 2 every 2nd frame is one render per frame); they go round-robin to
//!   the visible groups, the oldest picture first ([`pick_renders`]), so with more mirrors in view
//!   than the budget each updates less often instead of falling back to the static bitmap. A
//!   group waiting for its turn or out of view keeps its last picture. The switch is
//!   `Camera::is_active`, never the removal of `Camera3d`. Members that are hidden (a model keeps
//!   mutually exclusive variants) do not count for the pose, the distance and the view test.
//! * **Parity.** [`sync_mirror_camera_look`] copies `Hdr` and `DistanceFog` from the
//!   [`MainCamera`]; the mirror camera does **not** tonemap (the picture is scene-linear and is
//!   tonemapped once, by the main camera, when it is shown) and carries no colour grading for the
//!   same reason. Mirror cameras never carry [`MainCamera`], so every query that means the
//!   player's view skips them.

use bevy::camera::{Hdr, RenderTarget};
use bevy::pbr::StandardMaterial;
use bevy::prelude::*;
use bevy::render::view::Tonemapping;

#[cfg(test)]
use opts_core::formats::vehicles::BusMirrorCamera;

use crate::glass::{surface_kind, SurfaceKind};
use crate::light_quality::{LightQuality, LightQualitySet};
use crate::vehicle::BusVehicle;
use crate::vehicle_sim::PlayerVehicle;
use crate::MainCamera;

mod maths;
mod settings;
mod state;

pub use maths::*;
pub use settings::*;
pub use state::*;

// ---------------------------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------------------------

mod spawn;
pub use spawn::spawn_mirror_cameras;

mod schedule;
pub use schedule::{mirror_in_view, pick_renders, renders_in_frame, MIRROR_MIN_ANGLE};

mod update;
pub use update::{sync_mirror_camera_look, update_mirror_cameras};

mod ambient;
pub use ambient::{
    register_ambient_panes, update_ambient_mirrors, AmbientCamera, AmbientPane,
    AMBIENT_EVERY_FACTOR, AMBIENT_MIN_FACING, AMBIENT_PICTURE, AMBIENT_RANGE_M,
};

mod virtual_view;
pub use virtual_view::{
    assign_slots, stream_size, toggle_virtual_mirrors, MirrorFeed, MirrorSlot, VirtualMirrors,
    SIDE_MIRROR_FRONT_BAND_M, SIDE_MIRROR_MIN_X_M, SLOT_COUNT,
};

/// Registers the mirror cameras.
#[derive(Default)]
pub struct MirrorPlugin;

impl Plugin for MirrorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LightQuality>()
            .insert_resource(MirrorSettings::from_env())
            .init_resource::<MirrorFrame>()
            .init_resource::<MirrorStats>()
            .init_resource::<VirtualMirrors>()
            .add_systems(
                Update,
                spawn_mirror_cameras
                    .after(LightQualitySet)
                    .run_if(resource_exists::<Assets<Image>>)
                    .run_if(resource_exists::<Assets<StandardMaterial>>)
                    .run_if(resource_exists::<Assets<Mesh>>),
            )
            .add_systems(
                Update,
                toggle_virtual_mirrors
                    .run_if(crate::reporting::not_modal_active)
                    .run_if(resource_exists::<crate::camera::CameraState>)
                    .run_if(resource_exists::<crate::camera::CameraAvailability>),
            )
            .add_systems(
                Update,
                register_ambient_panes.run_if(resource_exists::<Assets<Mesh>>),
            )
            .add_systems(
                PostUpdate,
                (
                    update_mirror_cameras,
                    update_ambient_mirrors,
                    sync_mirror_camera_look,
                )
                    .chain()
                    .before(bevy::transform::TransformSystems::Propagate),
            );
    }
}

#[cfg(test)]
mod tests;
