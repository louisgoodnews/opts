//! Crate-root re-exports: the public names of the modules, so that paths such as `opts_bevy::ActiveCab` stay.

pub use crate::action_keys::{resolve as action_keys_of, ActionKeys};
pub use crate::ai_variables::{AiVariables, VariableSource};
pub use crate::articulation::{
    articulation_system, joint_beta_deg, ArticulatedChain, ArticulatedSegment, ArticulationPlugin,
    SegmentGround,
};
pub use crate::audio::{
    AudioSettings, AudioStats, Distance as SoundDistance, OptsAudioPlugin, PcmClip, SoundBus,
    SoundEmitter,
};
pub use crate::cab_switch::ActiveCab;
pub use crate::camera::{
    camera_hint_text, scale_free_roam_speed, CameraAvailability, CameraMode, CameraState,
    MainCamera, OptsCameraPlugin, Viewpoint, ViewpointRequest, ViewpointRow, ViewpointState,
    FREE_ROAM_MIN_SPEED,
};
pub use crate::consist_scripts::{ConsistUnit, ScriptConsist, ScriptPart};
pub use crate::coords::{
    bevy_quat_to_omsi_euler, bevy_to_omsi_pos, bevy_to_omsi_transform,
    camera_preset_to_bevy_transform, omsi_euler_to_bevy_quat, omsi_to_bevy_pos,
    omsi_to_bevy_transform, sync_bevy_to_omsi_transforms, sync_coordinate_transforms,
    sync_omsi_to_bevy_transforms, OmsiCoords, OmsiTransform,
};
pub use crate::environment::{
    light_attenuation, CalendarDate, ChronoRebuild, EnvironmentChange, EnvironmentPlugin,
    EnvironmentSet, LightAttenuation, MapDateChanged, MapDatePolicy, SessionEnvironment,
    WeatherKind, WeatherSky,
};
pub use crate::human::{
    apply_pose, hip_offset, pose_rotations, rest_pivots, seat_hip_lift, seated_rotations,
    seated_thigh_angle, spawn_human, sync_player_head, AlignEyes, DriverHead, Human, HumanHead,
    HumanPose, HumanRig, PlayerHead, HUMAN_BONES,
};
pub use crate::install::OmsiInstall;
pub use crate::light_cones::{
    ConeKind, ConeLevel, ConePool, ConeStats, LightCone, LightConePlugin, LightConeRenderPlugin,
};
pub use crate::light_rooms::{CabinView, LightRoomsPlugin, NoMapLighting, VehicleFootprint};
pub use crate::loader::{
    DdsAssetLoader, O3dAssetLoader, O3dLoaderError, OptsImageLoader, TextureLoaderError,
    XAssetLoader, XLoaderError,
};
pub use crate::material::{
    o3d_material_to_standard_material, specular_power_to_roughness, x_material_to_standard_material,
};
pub use crate::mesh::{
    o3d_to_bevy_mesh, o3d_to_bevy_meshes_by_material, parse_mesh_file, tree_billboard_material,
    tree_billboard_mesh, x_mesh_to_bevy_meshes, x_mesh_to_combined_bevy_mesh,
    x_submesh_to_bevy_mesh,
};
pub use crate::model::{
    attach_model_parts, load_decoded_meshes, ContentCache, ContentLoader, ContentStats, ModelPart,
    TextureOverride,
};
pub use crate::mouse_driving::MouseDriving;
pub use crate::plugin::{
    OptsAssetPlugin, OptsCoordsPlugin, OptsMaterialPlugin, OptsMeshPlugin, OptsSceneryPlugin,
    OptsVehiclePlugin,
};
pub use crate::position_tag::{position_tag, set_loading_tile, set_position_tag};
pub use crate::rail_controls::RailControls;
pub use crate::scenery::{
    extrude_spline_profile, extrude_spline_segment, spawn_scenery_object, spline_stations,
    SceneryLoaderError, SceneryObjectAsset, ScoAssetLoader, SliAssetLoader, SplineProfileAsset,
};
pub use crate::scenery_runtime::{
    advance_scenery, bind_alpha_scale, build_scenery_object, SceneryAlphaScale, SceneryClock,
    SceneryDrawDistance, SceneryDrawRoot, SceneryLodBias, SceneryLodMode, SceneryMeshRule,
    SceneryObjectRuntime, SceneryRuntimePlugin, SceneryRuntimeSettings, SceneryRuntimeStats,
    SceneryScripts, SceneryVariableOverrides, ShadowDetail,
};
pub use crate::terrain::{
    build_cut_terrain_mesh, build_terrain_mesh, carve_terrain, spline_cut_triangles,
    spline_ground_triangles, terrain_height, terrain_material, CarveStats, GroundTriangle,
    TerrainCut, TileTerrain, CARVE_COVER_M, CARVE_EPSILON_M, TERRAIN_CELL_M, TILE_SIZE_M,
};
pub use crate::terrain_query::GroundElevationQuery;
pub use crate::texture::{
    dds_to_bevy_image, image_to_bevy_image, load_bevy_image, sniff_image_format, ImageFormat,
    TextureError,
};
pub use crate::texture_filter::{
    current_object_sampler, object_sampler, set_texture_filtering, texture_filtering,
    TextureFiltering,
};
pub use crate::vehicle::{spawn_vehicle_model, BusVehicle};
pub use crate::vehicle_anim::{
    anim_steps, anim_steps_with, animation_matrix, vehicle_animation_system, AnimStep,
    MeshAnimation, MeshPivot, VehicleAnimationPlugin, SWAP_YZ_ROWS,
};
pub use crate::vehicle_glow::{
    choose_headlights, spawn_vehicle_glows, spawn_vehicle_glows_with_cones, VehicleGlow,
    VehicleGlowPlugin, HEADLIGHT_POOL,
};
pub use crate::vehicle_lights::{
    vehicle_alpha_scale_system, vehicle_light_map_system, vehicle_light_system, AlphaScaleBinding,
    LightMapBinding, LightSwitch, ScriptLight, VehicleLightPlugin,
};
pub use crate::vehicle_script::{
    script_drive_system, script_startup_system, vehicle_script_system, PassengerFeed,
    ScriptDriveEnabled, ScriptRadius, ScriptStartup, VehicleScriptPlugin, VehicleScripts,
};
pub use crate::vehicle_setup::{StartLights, VehicleSetup};
pub use crate::vehicle_sim::{
    vehicle_camera_system, vehicle_input_system, vehicle_physics_step_system,
    vehicle_transform_sync_system, wheel_transform_sync_system, PlayerVehicle, VehicleCameraMode,
    VehicleCameraRig, VehicleController, VehicleInputConfig, WheelSide, WheelVisual,
};
pub use crate::vehicle_triggers::{
    dik_to_keycode, hover_system, script_key_trigger_system, script_mouse_trigger_system,
    HotSpotPicker, HoveredControl, KeyChord, MouseTrigger, SceneryClicked, SceneryMouseTrigger,
    ScriptKeyBase, ScriptKeyBinding, ScriptKeyBindings, ScriptedClick, ScriptedClicks,
    VehicleTriggerPlugin, CABIN_LIGHT_TOGGLE, DOOR_SIDE_TOGGLE, OPTS_RESERVED_KEYS,
};
pub use crate::view_scope::{apply_view_scope, camera_is_inside, ViewScope, ViewScopePlugin};
pub use crate::water::{build_water_mesh, water_material};
pub use crate::weather::{WeatherCatalogue, WeatherConditions, WeatherEffects, WeatherVars};
