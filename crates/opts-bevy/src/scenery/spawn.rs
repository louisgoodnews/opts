//! Spawning of an OMSI scenery object as a Bevy entity hierarchy.

use bevy::asset::Handle;
use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
use bevy::ecs::system::Commands;
use bevy::mesh::{Mesh, Mesh3d};
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::transform::components::Transform;
use opts_core::formats::scenery::SceneryObject;

use crate::coords::OmsiTransform;

/// Spawns an OMSI scenery object entity hierarchy in Bevy.
///
/// Creates a parent entity carrying an [`OmsiTransform`] and `Transform`/`Visibility`,
/// along with child entities for visual 3D mesh attachments.
pub fn spawn_scenery_object(
    commands: &mut Commands,
    sco: &SceneryObject,
    transform: Transform,
    mesh_handles: &[Handle<Mesh>],
    material_handles: &[Handle<StandardMaterial>],
) -> Entity {
    let name = sco
        .friendly_name
        .clone()
        .unwrap_or_else(|| "SceneryObject".to_string());

    let parent = commands
        .spawn((
            Name::new(name),
            transform,
            OmsiTransform::from_bevy_transform(&transform),
        ))
        .id();

    for (i, mesh_def) in sco.meshes.iter().enumerate() {
        if let Some(mesh_handle) = mesh_handles.get(i) {
            let mat_handle = material_handles.get(i).cloned().unwrap_or_default();

            let child = commands
                .spawn((
                    Mesh3d(mesh_handle.clone()),
                    MeshMaterial3d(mat_handle),
                    Transform::IDENTITY,
                ))
                .insert(Name::new(format!("Mesh_{i}_{}", mesh_def.file)))
                .id();

            commands.entity(parent).add_children(&[child]);
        }
    }

    parent
}
