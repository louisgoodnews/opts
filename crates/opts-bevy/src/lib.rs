//! # OPTS Bevy (`opts-bevy`)
//!
//! Bevy engine integration, coordinate conversion bridges, and asset loaders
//! for Open Public Transport Simulator (OPTS).
//!
//! This crate bridges pure OMSI data structures parsed by `opts-core` into native
//! Bevy ECS components and rendering resources (`Mesh`, `Material`, `AssetLoader`).
//!
//! ## Coordinate Conversions
//!
//! OMSI meshes use a **left-handed Y-up** coordinate system (+X right, +Y up, +Z forward).
//! Bevy uses a **right-handed Y-up** coordinate system (+X right, +Y up, -Z forward, +Z back).
//!
//! See [`coordinates.md`](../../../docs/src/architecture/coordinates.md) for full details.

#![deny(unsafe_code)]

pub mod loader;
pub mod material;
pub mod mesh;
pub mod plugin;
pub mod texture;

pub use loader::{
    DdsAssetLoader, O3dAssetLoader, O3dLoaderError, OptsImageLoader, TextureLoaderError,
    XAssetLoader, XLoaderError,
};
pub use material::{
    o3d_material_to_standard_material, specular_power_to_roughness, x_material_to_standard_material,
};
pub use mesh::{
    o3d_to_bevy_mesh, x_mesh_to_bevy_meshes, x_mesh_to_combined_bevy_mesh, x_submesh_to_bevy_mesh,
};
pub use plugin::{OptsAssetPlugin, OptsMaterialPlugin, OptsMeshPlugin};
pub use texture::{
    dds_to_bevy_image, image_to_bevy_image, load_bevy_image, sniff_image_format, ImageFormat,
    TextureError,
};
