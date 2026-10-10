//! Vehicle entity spawning, camera projection, and ECS integration for Bevy.
//!
//! Bridges OMSI vehicle manifests ([`BusManifest`]) and 3D rendering configurations
//! ([`ModelConfig`]) into native Bevy entity hierarchies with visual mesh attachments,
//! material slots, interior/exterior light sources, and camera preset views.

use bevy::prelude::*;
use opts_core::formats::vehicles::{
    bus::{BusCamera, BusManifest},
    model::{InteriorLight, ModelConfig, Spotlight},
};

use crate::coords::{camera_preset_to_bevy_transform, omsi_to_bevy_pos, OmsiTransform};

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
    /// Passenger camera presets of coupled trailer segments, with the segment entity whose
    /// frame each preset is defined in (articulated buses and trams).
    pub trailer_pax_cameras: Vec<(Entity, BusCamera)>,
    /// Exterior orbit camera focus center in OMSI coordinates.
    pub outside_camera_center: Option<Vec3>,
    /// Index of the standard/default driver camera.
    pub std_camera_index: usize,
    /// Currently active camera preset index.
    pub active_camera_index: usize,
}

impl BusVehicle {
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

/// Spawns an OMSI bus vehicle entity hierarchy in Bevy.
///
/// Creates a root entity containing [`BusVehicle`], [`OmsiTransform`], and [`SpatialBundle`],
/// along with child entities for:
/// - Visual 3D mesh attachments from the primary LOD group.
/// - Unscoped and mesh-scoped spotlights (headlights) converted to [`SpotLightBundle`].
/// - Unscoped and mesh-scoped interior lights converted to [`PointLightBundle`].
///
/// # Arguments
/// * `commands` - Bevy entity commands buffer.
/// * `manifest` - Vehicle manifest parsed from a `.bus` file.
/// * `model` - Vehicle rendering manifest parsed from a `model.cfg` file.
/// * `transform` - World transform where the bus root should be spawned.
/// * `mesh_handles` - Ordered slice of Bevy mesh handles corresponding to meshes in the primary LOD tier.
/// * `material_handles` - Ordered slice of Bevy material handles corresponding to mesh slots.
pub fn spawn_vehicle_model(
    commands: &mut Commands,
    manifest: &BusManifest,
    model: &ModelConfig,
    transform: Transform,
    mesh_handles: &[Handle<Mesh>],
    material_handles: &[Handle<StandardMaterial>],
) -> Entity {
    let friendly_name = manifest
        .friendly_name
        .as_ref()
        .map(|f| format!("{} {}", f.manufacturer, f.model))
        .unwrap_or_else(|| "OMSI Bus".to_string());

    let std_cam_idx = manifest.std_camera_index.unwrap_or(0);

    let vehicle_component = BusVehicle {
        friendly_name: friendly_name.clone(),
        mass_tonnes: manifest.mass_tonnes,
        driver_cameras: manifest.driver_cameras.clone(),
        pax_cameras: manifest.pax_cameras.clone(),
        trailer_pax_cameras: Vec::new(),
        outside_camera_center: manifest.outside_camera_center,
        std_camera_index: std_cam_idx,
        active_camera_index: std_cam_idx,
    };

    let parent = commands
        .spawn((
            Name::new(friendly_name),
            SpatialBundle::from_transform(transform),
            OmsiTransform::from_bevy_transform(&transform),
            vehicle_component,
        ))
        .id();

    // 1. Spawn primary LOD meshes
    if let Some(lod0) = model.lods.first() {
        for (i, mesh_def) in lod0.meshes.iter().enumerate() {
            if let Some(mesh_handle) = mesh_handles.get(i) {
                let mat_handle = material_handles.get(i).cloned().unwrap_or_default();

                let child = commands
                    .spawn(PbrBundle {
                        mesh: mesh_handle.clone(),
                        material: mat_handle,
                        transform: Transform::IDENTITY,
                        ..Default::default()
                    })
                    .insert(Name::new(format!("Mesh_{i}_{}", mesh_def.path)))
                    .id();

                commands.entity(parent).push_children(&[child]);
            }
        }
    }

    // 2. Spawn interior lights
    for (i, light) in model.interior_lights.iter().enumerate() {
        spawn_interior_light_child(commands, parent, i, light);
    }

    // Also check mesh-scoped interior lights
    if let Some(lod0) = model.lods.first() {
        for (m_idx, mesh_def) in lod0.meshes.iter().enumerate() {
            for (i, light) in mesh_def.interior_lights.iter().enumerate() {
                spawn_interior_light_child(commands, parent, m_idx * 100 + i, light);
            }
        }
    }

    // 3. Spawn spotlights / headlights
    for (i, spot) in model.spotlights.iter().enumerate() {
        spawn_spotlight_child(commands, parent, i, spot);
    }

    if let Some(lod0) = model.lods.first() {
        for (m_idx, mesh_def) in lod0.meshes.iter().enumerate() {
            for (i, spot) in mesh_def.spotlights.iter().enumerate() {
                spawn_spotlight_child(commands, parent, m_idx * 100 + i, spot);
            }
        }
    }

    parent
}

fn spawn_interior_light_child(
    commands: &mut Commands,
    parent: Entity,
    index: usize,
    light: &InteriorLight,
) {
    let pos_bevy = omsi_to_bevy_pos(light.pos);
    let color = Color::srgb_u8(light.color_rgb[0], light.color_rgb[1], light.color_rgb[2]);

    let child = commands
        .spawn(PointLightBundle {
            point_light: PointLight {
                color,
                range: light.range,
                radius: 0.1,
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_translation(pos_bevy),
            ..default()
        })
        .insert(Name::new(format!(
            "InteriorLight_{index}_{}",
            light.variable
        )))
        .id();

    commands.entity(parent).push_children(&[child]);
}

fn spawn_spotlight_child(commands: &mut Commands, parent: Entity, index: usize, spot: &Spotlight) {
    let pos_bevy = omsi_to_bevy_pos(spot.pos);
    let dir_bevy = omsi_to_bevy_pos(spot.dir).normalize_or_zero();
    let rotation = if dir_bevy.length_squared() > 1e-4 {
        Quat::from_rotation_arc(Vec3::NEG_Z, dir_bevy)
    } else {
        Quat::IDENTITY
    };

    let color = Color::srgb_u8(spot.color_rgb[0], spot.color_rgb[1], spot.color_rgb[2]);

    let child = commands
        .spawn(SpotLightBundle {
            spot_light: SpotLight {
                color,
                range: spot.range,
                inner_angle: spot.inner_cone.to_radians(),
                outer_angle: spot.outer_cone.to_radians(),
                shadows_enabled: true,
                ..default()
            },
            transform: Transform {
                translation: pos_bevy,
                rotation,
                scale: Vec3::ONE,
            },
            ..default()
        })
        .insert(Name::new(format!("Spotlight_{index}")))
        .id();

    commands.entity(parent).push_children(&[child]);
}
