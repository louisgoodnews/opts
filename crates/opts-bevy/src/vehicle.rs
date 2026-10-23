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
/// Creates a root entity containing [`BusVehicle`], [`OmsiTransform`], and `Transform`/`Visibility`,
/// along with child entities for:
/// - Visual 3D mesh attachments from the primary LOD group.
/// - Unscoped and mesh-scoped spotlights (headlights) converted to `SpotLight` components.
/// - Unscoped and mesh-scoped interior lights converted to `PointLight` components.
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
        mirror_cameras: manifest.mirror_cameras.clone(),
        trailer_pax_cameras: Vec::new(),
        outside_camera_center: manifest.outside_camera_center,
        std_camera_index: std_cam_idx,
        active_camera_index: std_cam_idx,
    };

    let parent = commands
        .spawn((
            Name::new(friendly_name),
            transform,
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
                    .spawn((
                        Mesh3d(mesh_handle.clone()),
                        MeshMaterial3d(mat_handle),
                        Transform::IDENTITY,
                    ))
                    .insert(Name::new(format!("Mesh_{i}_{}", mesh_def.path)))
                    .id();

                commands.entity(parent).add_children(&[child]);
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

    // 3. Spawn spotlights / headlights, numbered in declaration order for `Spot_Select`
    // (unscoped ones first, then those of the meshes).
    let mesh_spots = model
        .lods
        .first()
        .into_iter()
        .flat_map(|lod| lod.meshes.iter().flat_map(|m| m.spotlights.iter()));
    let front_y = headlight_front_y(model);
    for (i, spot) in model.spotlights.iter().chain(mesh_spots).enumerate() {
        spawn_spotlight_child(commands, parent, i, spot, front_y);
    }

    // 4. Light effects (`[light_enh]`, `[light_enh_2]`) as glow sprites under the root. When the
    // model has `[spotlight]`s their cones are the headlight cones; otherwise the
    // `[light_enh_2]` headlights get them (the same rule AI and parked vehicles follow).
    let has_spots = !model.spotlights.is_empty()
        || model
            .lods
            .first()
            .is_some_and(|lod| lod.meshes.iter().any(|m| !m.spotlights.is_empty()));
    crate::vehicle_glow::spawn_vehicle_glows_with_cones(
        commands, parent, parent, model, !has_spots,
    );

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

    let point = PointLight {
        color,
        range: light.range,
        radius: 0.1,
        shadow_maps_enabled: false,
        ..default()
    };
    let switch = ScriptLight {
        owner: parent,
        switch: LightSwitch::Variable(light.variable.clone()),
        base_intensity: point.intensity,
    };
    // Cabin lights are on their own render layer: the main camera only includes it in an
    // interior view or close to the vehicle (`light_rooms`). Lights only, no meshes.
    let child = commands
        .spawn((
            point,
            switch,
            Transform::from_translation(pos_bevy),
            RenderLayers::layer(crate::light_rooms::CABIN_LAYER),
        ))
        .insert(Name::new(format!(
            "InteriorLight_{index}_{}",
            light.variable
        )))
        .id();

    commands.entity(parent).add_children(&[child]);
}

/// Where the headlights of a model are along the vehicle (OMSI +y): the front-most forward-facing
/// white `[light_enh_2]`; `None` when the model has none.
#[must_use]
fn headlight_front_y(model: &ModelConfig) -> Option<f32> {
    crate::vehicle_glow::detail_lights(model)
        .filter(|l| crate::vehicle_glow::is_headlight(l))
        .map(|l| l.position.y)
        .reduce(f32::max)
}

/// The place a `[spotlight]` shines from. Some models (the MAN Lion's City) put the emitters of
/// the beams 3 m behind the front, inside the body behind the driver's cabin: OMSI does not mind
/// (nothing casts shadows there), but here the beam's shadow map fills the passenger area with hard
/// shadows and a flooded floor. A forward-pointing emitter behind the headlights is moved forward
/// to them.
#[must_use]
fn spot_emitter(spot: &Spotlight, front_y: Option<f32>) -> Vec3 {
    let mut pos = spot.pos;
    if let Some(front) = front_y {
        if spot.dir.y > 0.5 && pos.y < front - 0.5 {
            pos.y = front;
        }
    }
    pos
}

fn spawn_spotlight_child(
    commands: &mut Commands,
    parent: Entity,
    index: usize,
    spot: &Spotlight,
    front_y: Option<f32>,
) {
    let pos_bevy = omsi_to_bevy_pos(spot_emitter(spot, front_y));
    let dir_bevy = omsi_to_bevy_pos(spot.dir).normalize_or_zero();
    let rotation = if dir_bevy.length_squared() > 1e-4 {
        Quat::from_rotation_arc(Vec3::NEG_Z, dir_bevy)
    } else {
        Quat::IDENTITY
    };

    let color = Color::srgb_u8(spot.color_rgb[0], spot.color_rgb[1], spot.color_rgb[2]);

    let light = SpotLight {
        color,
        range: spot.range,
        inner_angle: spot.inner_cone.to_radians(),
        outer_angle: spot.outer_cone.to_radians(),
        shadow_maps_enabled: true,
        ..default()
    };
    let switch = ScriptLight {
        owner: parent,
        switch: LightSwitch::Spot(index),
        base_intensity: light.intensity,
    };
    let child = commands
        .spawn((
            light,
            switch,
            Transform {
                translation: pos_bevy,
                rotation,
                scale: Vec3::ONE,
            },
        ))
        .insert(Name::new(format!("Spotlight_{index}")))
        .id();
    // The visible beam: a child of the light, so it points where the light points. Its
    // brightness is the light's own (`Spot_Select`).
    crate::light_cones::spawn_light_cone(
        commands,
        child,
        crate::light_cones::LightCone {
            kind: crate::light_cones::ConeKind::Headlight,
            shape: crate::light_cones::spotlight_cone_shape(spot),
            level: crate::light_cones::ConeLevel::Spot(child),
        },
    );

    commands.entity(parent).add_children(&[child]);
}

#[cfg(test)]
mod emitter_tests {
    use super::*;

    fn spot(y: f32, dir_y: f32) -> Spotlight {
        Spotlight {
            pos: Vec3::new(0.0, y, 1.1),
            dir: Vec3::new(0.0, dir_y, -0.3),
            color_rgb: [255; 3],
            range: 100.0,
            inner_cone: 0.0,
            outer_cone: 30.0,
        }
    }

    #[test]
    fn a_forward_emitter_inside_the_body_moves_to_the_headlights() {
        assert_eq!(spot_emitter(&spot(2.5, 1.0), Some(5.9)).y, 5.9);
        // At the front already, pointing backwards, or without headlights: unchanged.
        assert_eq!(spot_emitter(&spot(5.8, 1.0), Some(5.9)).y, 5.8);
        assert_eq!(spot_emitter(&spot(2.5, -1.0), Some(5.9)).y, 2.5);
        assert_eq!(spot_emitter(&spot(2.5, 1.0), None).y, 2.5);
    }
}
