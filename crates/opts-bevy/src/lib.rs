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

pub mod camera;
pub mod control;
pub mod coords;
pub mod descramble_cache;
pub mod ground;
pub mod ground_material;
pub mod hot_reload;
pub mod human;
pub mod install;
pub mod loader;
pub mod material;
pub mod mesh;
pub mod model;
pub mod plugin;
pub mod reporting;
pub mod scenery;
pub mod terrain;
pub mod text_texture;
pub mod texture;
pub mod vehicle;
pub mod vehicle_script;
pub mod vehicle_sim;

pub use camera::{
    camera_hint_text, scale_free_roam_speed, CameraAvailability, CameraMode, CameraState,
    OptsCameraPlugin, FREE_ROAM_MIN_SPEED,
};
pub use coords::{
    bevy_quat_to_omsi_euler, bevy_to_omsi_pos, bevy_to_omsi_transform,
    camera_preset_to_bevy_transform, omsi_euler_to_bevy_quat, omsi_to_bevy_pos,
    omsi_to_bevy_transform, sync_bevy_to_omsi_transforms, sync_coordinate_transforms,
    sync_omsi_to_bevy_transforms, OmsiCoords, OmsiTransform,
};
pub use human::{
    apply_pose, hip_offset, spawn_human, Human, HumanHead, HumanPose, HumanRig, HUMAN_BONES,
};
pub use install::OmsiInstall;
pub use loader::{
    DdsAssetLoader, O3dAssetLoader, O3dLoaderError, OptsImageLoader, TextureLoaderError,
    XAssetLoader, XLoaderError,
};
pub use material::{
    o3d_material_to_standard_material, specular_power_to_roughness, x_material_to_standard_material,
};
pub use mesh::{
    o3d_to_bevy_mesh, o3d_to_bevy_meshes_by_material, parse_mesh_file, tree_billboard_mesh,
    x_mesh_to_bevy_meshes, x_mesh_to_combined_bevy_mesh, x_submesh_to_bevy_mesh,
};
pub use model::{
    attach_model_parts, ContentCache, ContentLoader, ContentStats, ModelPart, TextureOverride,
};
pub use plugin::{
    OptsAssetPlugin, OptsCoordsPlugin, OptsMaterialPlugin, OptsMeshPlugin, OptsSceneryPlugin,
    OptsVehiclePlugin,
};
pub use scenery::{
    extrude_spline_profile, extrude_spline_segment, spawn_scenery_object, SceneryLoaderError,
    SceneryObjectAsset, ScoAssetLoader, SliAssetLoader, SplineProfileAsset,
};
pub use terrain::{
    build_terrain_mesh, carve_terrain_under_roads, road_footprint, sample_terrain,
    terrain_material, RoadFootprint, TERRAIN_CELL_M, TILE_SIZE_M,
};
pub use texture::{
    dds_to_bevy_image, image_to_bevy_image, load_bevy_image, sniff_image_format, ImageFormat,
    TextureError,
};
pub use vehicle::{spawn_vehicle_model, BusVehicle};
pub use vehicle_script::{vehicle_script_system, VehicleScriptPlugin, VehicleScripts};
pub use vehicle_sim::{
    vehicle_camera_system, vehicle_input_system, vehicle_physics_step_system,
    vehicle_transform_sync_system, wheel_transform_sync_system, PlayerVehicle, VehicleCameraMode,
    VehicleCameraRig, VehicleController, VehicleInputConfig, WheelSide, WheelVisual,
};
