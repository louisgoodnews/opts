//! Spawning the vehicle model: the root, the mesh attachments and the lights of the model.

use super::*;

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
        trailer_driver_cameras: Vec::new(),
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
