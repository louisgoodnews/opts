//! Vehicle entity spawning, camera projection, and ECS integration for Bevy.
//!
//! Bridges OMSI vehicle manifests ([`BusManifest`]) and 3D rendering configurations
//! ([`ModelConfig`]) into native Bevy entity hierarchies with visual mesh attachments,
//! material slots, interior/exterior light sources, and camera preset views.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use opts_core::formats::vehicles::{
    bus::{BusCamera, BusManifest},
    model::{InteriorLight, ModelConfig, Spotlight},
};

use crate::coords::{camera_preset_to_bevy_transform, omsi_to_bevy_pos, OmsiTransform};
use crate::vehicle_lights::{LightSwitch, ScriptLight};

/// Component attached to a spawned OMSI bus vehicle root entity.
#[derive(Component, Debug, Clone)]
pub struct BusVehicle {
    /// Human-friendly display name (manufacturer and model).
    pub friendly_name: String,
    /// Curb mass of the vehicle in tonnes, if specified.
    pub mass_tonnes: Option<f32>,
    /// Driver view camera presets.
    pub driver_cameras: Vec<BusCamera>,
    /// Passenger compartment camera presets of the main segment.
    pub pax_cameras: Vec<BusCamera>,
    /// Mirror cameras of the `.bus` file: entry N renders the pane texture `reflexion<N>.bmp`.
    pub mirror_cameras: Vec<opts_core::formats::vehicles::BusMirrorCamera>,
    /// Passenger camera presets of coupled trailer segments, with the segment entity whose
    /// frame each preset is defined in (articulated buses and trams).
    pub trailer_pax_cameras: Vec<(Entity, BusCamera)>,
    /// Driver camera presets of the cabs of coupled units (the second head of a tram, the end
    /// cars of a traction), with the unit entity whose frame each is defined in.
    pub trailer_driver_cameras: Vec<(Entity, BusCamera)>,
    /// Exterior orbit camera focus center in OMSI coordinates.
    pub outside_camera_center: Option<Vec3>,
    /// Index of the standard/default driver camera.
    pub std_camera_index: usize,
    /// Currently active camera preset index.
    pub active_camera_index: usize,
}

impl BusVehicle {
    /// Number of driver cameras over all cabs: the head's first, then those of the coupled units
    /// in order.
    #[must_use]
    pub fn driver_camera_count(&self) -> usize {
        self.driver_cameras.len() + self.trailer_driver_cameras.len()
    }

    /// Driver camera `index` of the flattened list, with the unit entity it belongs to (`None` =
    /// the vehicle root itself).
    #[must_use]
    pub fn driver_camera(&self, index: usize) -> Option<(Option<Entity>, &BusCamera)> {
        if let Some(cam) = self.driver_cameras.get(index) {
            return Some((None, cam));
        }
        self.trailer_driver_cameras
            .get(index.checked_sub(self.driver_cameras.len())?)
            .map(|(e, c)| (Some(*e), c))
    }

    /// The first driver camera of the cab in unit `unit` (`None` = the head), if it has one.
    #[must_use]
    pub fn first_driver_camera_of(&self, unit: Option<Entity>) -> Option<usize> {
        match unit {
            None => (!self.driver_cameras.is_empty()).then_some(0),
            Some(e) => self
                .trailer_driver_cameras
                .iter()
                .position(|(u, _)| *u == e)
                .map(|i| i + self.driver_cameras.len()),
        }
    }

    /// The units that have a cab (the head first when it has driver cameras), each once, in
    /// order.
    #[must_use]
    pub fn cab_units(&self) -> Vec<Option<Entity>> {
        let mut out = Vec::new();
        if !self.driver_cameras.is_empty() {
            out.push(None);
        }
        for (e, _) in &self.trailer_driver_cameras {
            if !out.contains(&Some(*e)) {
                out.push(Some(*e));
            }
        }
        out
    }

    /// Number of passenger cameras over all segments: main segment first, then trailers in order.
    #[must_use]
    pub fn pax_camera_count(&self) -> usize {
        self.pax_cameras.len() + self.trailer_pax_cameras.len()
    }

    /// Passenger camera `index` of the flattened list, with the segment entity it belongs to
    /// (`None` = the vehicle root itself).
    #[must_use]
    pub fn pax_camera(&self, index: usize) -> Option<(Option<Entity>, &BusCamera)> {
        if let Some(cam) = self.pax_cameras.get(index) {
            return Some((None, cam));
        }
        self.trailer_pax_cameras
            .get(index - self.pax_cameras.len())
            .map(|(e, c)| (Some(*e), c))
    }

    /// Returns the active [`BusCamera`] preset if within bounds.
    #[must_use]
    pub fn current_camera(&self) -> Option<&BusCamera> {
        self.driver_cameras
            .get(self.active_camera_index)
            .or_else(|| {
                self.pax_cameras.get(
                    self.active_camera_index
                        .saturating_sub(self.driver_cameras.len()),
                )
            })
    }

    /// Computes the native Bevy [`Transform`] for the currently active camera preset.
    ///
    /// Evaluates the lateral $X$, longitudinal $Y$, and vertical $Z$ offsets with viewing
    /// directions via [`camera_preset_to_bevy_transform`].
    #[must_use]
    pub fn current_camera_bevy_transform(&self) -> Option<Transform> {
        let cam = self.current_camera()?;
        Some(camera_preset_to_bevy_transform(
            Vec3::new(cam.x, cam.y, cam.z),
            cam.dist,
            cam.yaw_deg,
            cam.pitch_deg,
        ))
    }
}

mod lights;
mod spawn;

pub use spawn::*;
use lights::*;

#[cfg(test)]
mod tests;
