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

pub mod ai_variables;
pub mod articulation;
pub mod audio;
pub mod camera;
pub mod capture;
pub mod control;
pub mod coords;
pub mod door_camera;
pub mod environment;
pub mod glass;
pub mod ground;
pub mod ground_material;
pub mod host_callbacks;
pub mod hot_reload;
pub mod human;
pub mod install;
pub mod instancing;
pub mod light_cones;
pub mod light_quality;
pub mod light_rooms;
pub mod loader;
pub mod material;
pub mod mesh;
pub mod mirrors;
pub mod model;
pub mod moon;
pub mod moon_material;
pub mod plugin;
pub mod position_tag;
pub mod reflections;
pub mod render_stats;
pub mod reporting;
pub mod route;
pub mod scene_trace;
pub mod scenery;
pub mod scenery_runtime;
pub mod sky_cube;
pub mod sun_rays;
pub mod terrain;
pub mod terrain_query;
pub mod text_texture;
pub mod texture;
pub mod texture_filter;
pub mod texture_sentinel;
pub mod vehicle;
pub mod vehicle_anim;
pub mod vehicle_glow;
pub mod vehicle_lights;
pub mod vehicle_remote;
pub mod vehicle_script;
pub mod vehicle_sim;
pub mod vehicle_triggers;
pub mod view_scope;
pub mod water;
pub mod weather;

pub use ai_variables::{AiVariables, VariableSource};
pub use articulation::{
    articulation_system, joint_beta_deg, ArticulatedChain, ArticulatedSegment, ArticulationPlugin,
    SegmentGround,
};
pub use audio::{
    AudioSettings, AudioStats, Distance as SoundDistance, OptsAudioPlugin, PcmClip, SoundBus,
    SoundEmitter,
};
pub use camera::{
    camera_hint_text, scale_free_roam_speed, CameraAvailability, CameraMode, CameraState,
    MainCamera, OptsCameraPlugin, Viewpoint, ViewpointRequest, ViewpointRow, ViewpointState,
    FREE_ROAM_MIN_SPEED,
};
pub use coords::{
    bevy_quat_to_omsi_euler, bevy_to_omsi_pos, bevy_to_omsi_transform,
    camera_preset_to_bevy_transform, omsi_euler_to_bevy_quat, omsi_to_bevy_pos,
    omsi_to_bevy_transform, sync_bevy_to_omsi_transforms, sync_coordinate_transforms,
    sync_omsi_to_bevy_transforms, OmsiCoords, OmsiTransform,
};
pub use environment::{
    light_attenuation, CalendarDate, ChronoRebuild, EnvironmentChange, EnvironmentPlugin,
    EnvironmentSet, LightAttenuation, MapDateChanged, MapDatePolicy, SessionEnvironment,
    WeatherKind, WeatherSky,
};
pub use human::{
    apply_pose, hip_offset, pose_rotations, rest_pivots, spawn_human, sync_player_head, AlignEyes,
    DriverHead, Human, HumanHead, HumanPose, HumanRig, PlayerHead, HUMAN_BONES,
};
pub use install::OmsiInstall;
pub use light_cones::{
    ConeKind, ConeLevel, ConePool, ConeStats, LightCone, LightConePlugin, LightConeRenderPlugin,
};
pub use light_rooms::{CabinView, LightRoomsPlugin, NoMapLighting, VehicleFootprint};
pub use loader::{
    DdsAssetLoader, O3dAssetLoader, O3dLoaderError, OptsImageLoader, TextureLoaderError,
    XAssetLoader, XLoaderError,
};
pub use material::{
    o3d_material_to_standard_material, specular_power_to_roughness, x_material_to_standard_material,
};
pub use mesh::{
    o3d_to_bevy_mesh, o3d_to_bevy_meshes_by_material, parse_mesh_file, tree_billboard_material,
    tree_billboard_mesh, x_mesh_to_bevy_meshes, x_mesh_to_combined_bevy_mesh,
    x_submesh_to_bevy_mesh,
};
pub use model::{
    attach_model_parts, load_decoded_meshes, ContentCache, ContentLoader, ContentStats, ModelPart,
    TextureOverride,
};
pub use plugin::{
    OptsAssetPlugin, OptsCoordsPlugin, OptsMaterialPlugin, OptsMeshPlugin, OptsSceneryPlugin,
    OptsVehiclePlugin,
};
pub use position_tag::{position_tag, set_loading_tile, set_position_tag};
pub use scenery::{
    extrude_spline_profile, extrude_spline_segment, spawn_scenery_object, spline_stations,
    SceneryLoaderError, SceneryObjectAsset, ScoAssetLoader, SliAssetLoader, SplineProfileAsset,
};
pub use scenery_runtime::{
    advance_scenery, bind_alpha_scale, build_scenery_object, SceneryAlphaScale, SceneryClock,
    SceneryDrawDistance, SceneryDrawRoot, SceneryLodMode, SceneryMeshRule, SceneryObjectRuntime,
    SceneryRuntimePlugin, SceneryRuntimeSettings, SceneryRuntimeStats, SceneryScripts,
    SceneryVariableOverrides,
};
pub use terrain::{
    build_terrain_mesh, carve_terrain, spline_ground_triangles, terrain_height, terrain_material,
    CarveStats, GroundTriangle, TileTerrain, CARVE_COVER_M, CARVE_EPSILON_M, TERRAIN_CELL_M,
    TILE_SIZE_M,
};
pub use terrain_query::GroundElevationQuery;
pub use texture::{
    dds_to_bevy_image, image_to_bevy_image, load_bevy_image, sniff_image_format, ImageFormat,
    TextureError,
};
pub use texture_filter::{
    current_object_sampler, object_sampler, set_texture_filtering, texture_filtering,
    TextureFiltering,
};
pub use vehicle::{spawn_vehicle_model, BusVehicle};
pub use vehicle_anim::{
    anim_steps, anim_steps_with, animation_matrix, vehicle_animation_system, AnimStep,
    MeshAnimation, MeshPivot, VehicleAnimationPlugin, SWAP_YZ_ROWS,
};
pub use vehicle_glow::{
    choose_headlights, spawn_vehicle_glows, spawn_vehicle_glows_with_cones, VehicleGlow,
    VehicleGlowPlugin, HEADLIGHT_POOL,
};
pub use vehicle_lights::{
    vehicle_alpha_scale_system, vehicle_light_map_system, vehicle_light_system, AlphaScaleBinding,
    LightMapBinding, LightSwitch, ScriptLight, VehicleLightPlugin,
};
pub use vehicle_script::{
    script_drive_system, script_startup_system, vehicle_script_system, ScriptDriveEnabled,
    ScriptRadius, ScriptStartup, VehicleScriptPlugin, VehicleScripts,
};
pub use vehicle_sim::{
    vehicle_camera_system, vehicle_input_system, vehicle_physics_step_system,
    vehicle_transform_sync_system, wheel_transform_sync_system, PlayerVehicle, VehicleCameraMode,
    VehicleCameraRig, VehicleController, VehicleInputConfig, WheelSide, WheelVisual,
};
pub use vehicle_triggers::{
    dik_to_keycode, script_key_trigger_system, script_mouse_trigger_system, KeyChord, MouseTrigger,
    ScriptKeyBase, ScriptKeyBinding, ScriptKeyBindings, ScriptedClick, ScriptedClicks,
    VehicleTriggerPlugin, OPTS_RESERVED_KEYS,
};
pub use view_scope::{apply_view_scope, camera_is_inside, ViewScope, ViewScopePlugin};
pub use water::{build_water_mesh, water_material};
pub use weather::{WeatherCatalogue, WeatherConditions, WeatherEffects, WeatherVars};

/// System set of the rail-bound player's track step: runs after the road physics and before the
/// articulation of the following units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, bevy::ecs::schedule::SystemSet)]
pub struct RailSet;
