//! Markers of wind-moved trees: a billboard or a mesh's foliage is [`WindTree`]; a mesh tree's
//! foliage is also [`MeshTree`].

use bevy::pbr::StandardMaterial;
use bevy::prelude::*;

/// Marks a tree whose material should move in the wind.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WindTree;

/// With [`WindTree`]: the mesh is a modelled tree's foliage, not a billboard (M-0144 phase C).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MeshTree;

/// Marks the root of a placed object modelled as a tree (M-0144 phase C): [`tag_mesh_tree_foliage`]
/// lets its cut-out and blended parts (the foliage) move in the wind; the opaque trunk stays.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MeshTreeRoot;

/// Gives the foliage of new [`MeshTreeRoot`]s [`WindTree`] and [`MeshTree`]: every descendant
/// part whose material is not opaque.
pub fn tag_mesh_tree_foliage(
    mut commands: Commands,
    roots: Query<Entity, Added<MeshTreeRoot>>,
    children: Query<&Children>,
    parts: Query<&MeshMaterial3d<StandardMaterial>, Without<WindTree>>,
    standard: Res<Assets<StandardMaterial>>,
) {
    let mut tagged = 0usize;
    for root in &roots {
        for entity in children.iter_descendants(root) {
            let Ok(material) = parts.get(entity) else {
                continue;
            };
            if standard
                .get(&material.0)
                .is_some_and(|m| !matches!(m.alpha_mode, AlphaMode::Opaque))
            {
                commands.entity(entity).insert((WindTree, MeshTree));
                tagged += 1;
            }
        }
    }
    if tagged > 0 {
        tracing::debug!(tagged, "mesh tree foliage moves in the wind");
    }
}
