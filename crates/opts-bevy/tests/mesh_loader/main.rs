use bevy::app::App;
use bevy::asset::AssetPlugin;
use bevy::math::{Vec2, Vec3};
use bevy::mesh::{Indices, Mesh, VertexAttributeValues};
use bevy::MinimalPlugins;

use opts_bevy::{
    o3d_to_bevy_mesh, x_mesh_to_bevy_meshes, x_mesh_to_combined_bevy_mesh, x_submesh_to_bevy_mesh,
    OptsMeshPlugin,
};
use opts_core::formats::o3d::{O3d, Vertex};
use opts_core::formats::x::{XFrame, XMaterial, XMesh, XSubMesh, XVertex};

mod o3d_colors;
mod o3d_conversion;
mod plugin;
mod x_meshes;
