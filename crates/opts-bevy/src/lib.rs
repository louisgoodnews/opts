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
pub mod mesh;
pub mod plugin;

pub use loader::{O3dAssetLoader, O3dLoaderError, XAssetLoader, XLoaderError};
pub use mesh::{
    o3d_to_bevy_mesh, x_mesh_to_bevy_meshes, x_mesh_to_combined_bevy_mesh, x_submesh_to_bevy_mesh,
};
pub use plugin::OptsMeshPlugin;
