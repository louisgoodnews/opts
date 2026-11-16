//! Light rooms: which lights the main camera sees, and the CPU rules that keep a street lamp
//! from lighting the saloon (`docs/src/architecture/lighting-system.md`, section 4).
//!
//! Bevy has no per-mesh light mask: a light's `RenderLayers` only decide which *views* it is in
//! (and which meshes cast its shadow), never which meshes it shades. So the rooms are
//! conventions and CPU rules:
//!
//! | Layer | Holds | Camera includes it |
//! | ---: | :--- | :--- |
//! | [`WORLD_LAYER`] 0 | everything (the default; no component needed) | always |
//! | [`CABIN_LAYER`] 1 | the player vehicle's `[interiorlight]` point lights (lights only) | in an interior view, or within [`CABIN_ENTER_M`] of the player vehicle ([`CABIN_LEAVE_M`] to leave) |
//!
//! * [`camera_light_layers_system`] keeps the main camera's `RenderLayers` at `{0}` or `{0, 1}`.
//!   Other cameras (the reflection prototype's mirror camera) have no layer component, so they
//!   render `{0}` and never pay for cabin lights.
//! * [`CabinView`] tells the map light pool that the camera is in an interior view (it dims the
//!   pool by [`INTERIOR_VIEW_SCALE`]) and, with the player vehicle's [`VehicleFootprint`],
//!   which lamps hang over the vehicle (they are skipped, [`over_vehicle`]).
//! * [`NoMapLighting`] marks objects with `[nomaplighting]` (v1: a marker only).

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

use crate::camera::{CameraMode, CameraState};
use crate::scenery_runtime::SceneryRuntimeSet;
use crate::vehicle_sim::{PlayerVehicle, VehicleCameraMode, VehicleCameraRig};

/// The default layer: the whole world.
pub const WORLD_LAYER: usize = 0;
/// The layer of the player vehicle's cabin lights.
pub const CABIN_LAYER: usize = 1;
/// The camera joins [`CABIN_LAYER`] when it comes this close to the player vehicle (m).
pub const CABIN_ENTER_M: f32 = 12.0;
/// The camera leaves [`CABIN_LAYER`] when it is this far from the player vehicle (m).
pub const CABIN_LEAVE_M: f32 = 15.0;
/// The map light pool's intensity factor while the camera is in an interior view, so a lamp
/// outside does not out-shine the cabin light ❓ (tune on screenshots).
pub const INTERIOR_VIEW_SCALE: f32 = 0.5;
/// Share of the sky's ambient light (the ambient colour and the sky cube) that reaches the
/// camera in an interior view: a cabin is lit from the windows and its own lamps, not by the
/// whole sky, and without this its surfaces look flat and washed out.
pub const INTERIOR_AMBIENT_SCALE: f32 = 0.45;
/// Lamps over the vehicle are skipped within this margin around its footprint (m).
pub const FOOTPRINT_MARGIN_M: f32 = 1.0;
mod ambient;
mod footprint;
mod layers;

pub use ambient::*;
pub use footprint::*;
pub use layers::*;

/// Marker on the root of an object with `[nomaplighting]` (not lit by map lights). Version 1
/// only records it; see `lighting-system.md` section 4.6.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct NoMapLighting;

/// Registers the light-room systems. They run before the scenery runtime so the map light pool
/// sees this frame's [`CabinView`].
#[derive(Default)]
pub struct LightRoomsPlugin;

impl Plugin for LightRoomsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CabinView>().add_systems(
            Update,
            (
                vehicle_footprint_system,
                camera_light_layers_system,
                interior_ambient_system,
            )
                .chain()
                .before(SceneryRuntimeSet),
        );
    }
}

#[cfg(test)]
mod tests;
