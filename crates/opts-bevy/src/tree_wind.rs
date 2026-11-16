//! Trees move in the wind (M-0144; `docs/src/architecture/vegetation-wind.md`).
//!
//! OMSI's trees stand still; this is an OPTS extension. A `[tree]` billboard is spawned with its
//! ordinary material and the [`WindTree`] marker; [`adopt_tree_material`] gives it the
//! [`TreeMaterial`] (the same material extended by a vertex stage, `shaders/tree_wind.wesl`, that
//! bends the crown along the wind in the main pass and in the depth and shadow passes). Without
//! [`TreeWindPlugin`] (headless runs) trees keep their material and stand still. The wind is a
//! A tree modelled as a mesh (its foliage cut-out cards in the `.o3d`) is marked with
//! [`MeshTreeRoot`]; [`tag_mesh_tree_foliage`] gives its non-opaque parts [`WindTree`] and
//! [`MeshTree`], whose material bends with the height above the object alone. The wind is a
//! uniform of the material: [`update_tree_wind`] writes it when the weather or the setting
//! changes; the time comes from the shader globals.

mod adopt;
mod markers;
mod material;
mod wind;

pub use adopt::adopt_tree_material;
pub use markers::{tag_mesh_tree_foliage, MeshTree, MeshTreeRoot, WindTree};
pub use material::{TreeMaterial, TreeWind, TreeWindUniform, BILLBOARD, MESH_TREE};
pub use wind::{update_tree_wind, wind_uniform, TreeWindState};

use bevy::asset::load_internal_asset;
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;

use material::SHADER;

/// Registers the tree material, its shader and the wind update.
#[derive(Default)]
pub struct TreeWindPlugin;

impl Plugin for TreeWindPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "shaders/tree_wind.wesl", Shader::from_wesl);
        app.add_plugins(MaterialPlugin::<TreeMaterial>::default())
            .init_resource::<TreeWindState>()
            .add_systems(
                Update,
                (tag_mesh_tree_foliage, update_tree_wind, adopt_tree_material).chain(),
            );
    }
}

#[cfg(test)]
mod tests;
