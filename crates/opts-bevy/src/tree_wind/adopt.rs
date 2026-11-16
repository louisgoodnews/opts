//! Adoption: new [`WindTree`]s get the tree material, extended by the wind.

use std::collections::HashMap;

use bevy::pbr::StandardMaterial;
use bevy::prelude::*;

use super::markers::{MeshTree, WindTree};
use super::material::{TreeMaterial, TreeWind, BILLBOARD, MESH_TREE};
use super::wind::TreeWindState;

/// New [`WindTree`]s with their material and whether they are a mesh tree's foliage.
type NewTrees<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static MeshMaterial3d<StandardMaterial>,
        Has<MeshTree>,
    ),
    Added<WindTree>,
>;

/// The tree material made for a base material, per kind of tree (mesh foliage or not).
type AdoptedMaterials = HashMap<(AssetId<StandardMaterial>, bool), Handle<TreeMaterial>>;

/// Gives new [`WindTree`]s the tree material: their own material, extended by the wind (one tree
/// material per base material, so trees of one texture share it).
pub fn adopt_tree_material(
    mut commands: Commands,
    trees: NewTrees,
    standard: Res<Assets<StandardMaterial>>,
    mut tree_materials: ResMut<Assets<TreeMaterial>>,
    state: Res<TreeWindState>,
    mut adopted: Local<AdoptedMaterials>,
) {
    for (entity, material, mesh_tree) in &trees {
        let key = (material.0.id(), mesh_tree);
        let handle = if let Some(h) = adopted.get(&key) {
            h.clone()
        } else {
            let Some(base) = standard.get(&material.0) else {
                continue;
            };
            let h = tree_materials.add(TreeMaterial {
                base: base.clone(),
                extension: TreeWind {
                    wind: state.wind,
                    shape: if mesh_tree { MESH_TREE } else { BILLBOARD },
                },
            });
            adopted.insert(key, h.clone());
            h
        };
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
    }
}
