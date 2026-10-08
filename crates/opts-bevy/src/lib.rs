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
//! - **OMSI Meshes (`.o3d`, `.x`)**: Left-handed Y-up (+X right, +Y up, +Z forward).
//! - **OMSI Entities & Vehicles (`.bus`, `.map`)**: Right-handed Z-up (+X right, +Y forward, +Z up).
//! - **Bevy Engine Space**: Right-handed Y-up (+X right, +Y up, -Z forward, +Z back).
//!
//! See [`coordinates.md`](../../../docs/src/architecture/coordinates.md) for full details.

#![deny(unsafe_code)]

pub mod coords;
pub mod loader;
pub mod material;
pub mod mesh;
pub mod plugin;
pub mod scenery;
pub mod texture;
pub mod vehicle;
pub mod vehicle_sim;

pub use coords::{
    bevy_quat_to_omsi_euler, bevy_to_omsi_pos, bevy_to_omsi_transform,
    camera_preset_to_bevy_transform, omsi_euler_to_bevy_quat, omsi_to_bevy_pos,
    omsi_to_bevy_transform, sync_bevy_to_omsi_transforms, sync_coordinate_transforms,
    sync_omsi_to_bevy_transforms, OmsiCoords, OmsiTransform,
};
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
pub use plugin::{
    OptsAssetPlugin, OptsCoordsPlugin, OptsMaterialPlugin, OptsMeshPlugin, OptsSceneryPlugin,
    OptsVehiclePlugin,
};
pub use scenery::{
    extrude_spline_profile, spawn_scenery_object, SceneryLoaderError, SceneryObjectAsset,
    ScoAssetLoader, SliAssetLoader, SplineProfileAsset,
};
pub use texture::{
    dds_to_bevy_image, image_to_bevy_image, load_bevy_image, sniff_image_format, ImageFormat,
    TextureError,
};
pub use vehicle::{spawn_vehicle_model, BusVehicle};
pub use vehicle_sim::{
    vehicle_camera_system, vehicle_input_system, vehicle_physics_step_system,
    vehicle_transform_sync_system, wheel_transform_sync_system, PlayerVehicle, VehicleCameraMode,
    VehicleCameraRig, VehicleController, VehicleInputConfig, WheelSide, WheelVisual,
};
