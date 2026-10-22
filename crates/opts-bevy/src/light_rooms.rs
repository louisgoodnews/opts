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

/// Bounding box of the player vehicle in its own frame (Bevy axes), from its mesh bounds.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct VehicleFootprint {
    /// Smallest corner.
    pub min: Vec3,
    /// Largest corner.
    pub max: Vec3,
}

impl VehicleFootprint {
    /// Distance from the vehicle-frame point `p` to the box (0 inside).
    #[must_use]
    pub fn distance(&self, p: Vec3) -> f32 {
        let d = (self.min - p).max(p - self.max).max(Vec3::ZERO);
        d.length()
    }
}

/// Whether the world point `lamp` hangs over the vehicle: horizontally inside its footprint grown
/// by `margin`, and not below the roof. `vehicle` is the vehicle root's transform.
#[must_use]
pub fn over_vehicle(
    lamp: Vec3,
    vehicle: &GlobalTransform,
    footprint: &VehicleFootprint,
    margin: f32,
) -> bool {
    let p = vehicle.affine().inverse().transform_point3(lamp);
    p.x >= footprint.min.x - margin
        && p.x <= footprint.max.x + margin
        && p.z >= footprint.min.z - margin
        && p.z <= footprint.max.z + margin
        && p.y >= footprint.max.y
}

/// Marker on the root of an object with `[nomaplighting]` (not lit by map lights). Version 1
/// only records it; see `lighting-system.md` section 4.6.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct NoMapLighting;

/// Whether the camera is in an interior view.
fn interior_view(
    state: Option<&CameraState>,
    rigs: &Query<&VehicleCameraRig, With<crate::MainCamera>>,
) -> bool {
    state.is_some_and(|s| matches!(s.mode, CameraMode::Cockpit | CameraMode::Passenger))
        || rigs
            .iter()
            .any(|r| r.mode == VehicleCameraMode::InteriorCockpit)
}

/// A player vehicle whose footprint is not known yet.
type Unmeasured = (With<PlayerVehicle>, Without<VehicleFootprint>);

/// Computes the [`VehicleFootprint`] of the player vehicle from the bounds of its mesh parts
/// (once they exist; retried until then).
pub fn vehicle_footprint_system(
    mut commands: Commands,
    vehicles: Query<(Entity, &GlobalTransform), Unmeasured>,
    children: Query<&Children>,
    parts: Query<(&bevy::camera::primitives::Aabb, &GlobalTransform), Without<PlayerVehicle>>,
) {
    for (vehicle, global) in &vehicles {
        let inverse = global.affine().inverse();
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for e in children.iter_descendants(vehicle) {
            let Ok((aabb, part)) = parts.get(e) else {
                continue;
            };
            let c = Vec3::from(aabb.center);
            let h = Vec3::from(aabb.half_extents);
            for corner in 0..8u8 {
                let s = Vec3::new(
                    if corner & 1 == 0 { -1.0 } else { 1.0 },
                    if corner & 2 == 0 { -1.0 } else { 1.0 },
                    if corner & 4 == 0 { -1.0 } else { 1.0 },
                );
                let local = inverse.transform_point3(part.transform_point(c + h * s));
                lo = lo.min(local);
                hi = hi.max(local);
            }
        }
        if lo.x <= hi.x {
            commands
                .entity(vehicle)
                .insert(VehicleFootprint { min: lo, max: hi });
        }
    }
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

/// Marks the camera whose [`AmbientLight`] [`interior_ambient_system`] wrote.
#[derive(Component)]
pub struct InteriorAmbient;

/// In an interior view the main camera gets its own, dimmer ambient light (a camera's
/// `AmbientLight` replaces the global one); outside it is taken off again.
pub fn interior_ambient_system(
    mut commands: Commands,
    view: Res<CabinView>,
    global: Option<Res<GlobalAmbientLight>>,
    cameras: Query<(Entity, Has<InteriorAmbient>), With<crate::MainCamera>>,
) {
    for (camera, marked) in &cameras {
        match (view.interior_view, global.as_deref()) {
            (true, Some(g)) => {
                commands.entity(camera).insert((
                    InteriorAmbient,
                    AmbientLight {
                        color: g.color,
                        brightness: g.brightness * INTERIOR_AMBIENT_SCALE,
                        affects_lightmapped_meshes: g.affects_lightmapped_meshes,
                    },
                ));
            }
            _ if marked => {
                commands
                    .entity(camera)
                    .remove::<(InteriorAmbient, AmbientLight)>();
            }
            _ => {}
        }
    }
}

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
