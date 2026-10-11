//! Bevy plugin registering OMSI mesh asset loaders, texture loaders, and coordinate systems.

use bevy::app::{App, Plugin, PostUpdate};
use bevy::asset::AssetApp;

use crate::coords::{sync_coordinate_transforms, OmsiCoords, OmsiTransform};
use crate::loader::{DdsAssetLoader, O3dAssetLoader, OptsImageLoader, XAssetLoader};

/// Bevy plugin that registers native asset loaders for OMSI `.o3d` and DirectX `.x` meshes.
///
/// # Usage
///
/// ```no_run
/// use bevy::prelude::*;
/// use opts_bevy::OptsMeshPlugin;
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(OptsMeshPlugin)
///     .run();
/// ```
#[derive(Default)]
pub struct OptsMeshPlugin;

impl Plugin for OptsMeshPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<O3dAssetLoader>()
            .init_asset_loader::<XAssetLoader>();
    }
}

/// Bevy plugin that registers native asset loaders for OMSI textures (`.dds`, `.bmp`, `.tga`).
///
/// # Usage
///
/// ```no_run
/// use bevy::prelude::*;
/// use opts_bevy::OptsMaterialPlugin;
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(OptsMaterialPlugin)
///     .run();
/// ```
#[derive(Default)]
pub struct OptsMaterialPlugin;

impl Plugin for OptsMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<DdsAssetLoader>()
            .init_asset_loader::<OptsImageLoader>();
    }
}

/// Bevy plugin that registers OMSI coordinate components and bidirectional transform sync systems.
///
/// Registers:
/// - [`OmsiCoords`] and [`OmsiTransform`] component reflection types.
/// - [`crate::coords::sync_omsi_to_bevy_transforms`] system in [`PostUpdate`].
/// - [`crate::coords::sync_bevy_to_omsi_transforms`] system in [`PostUpdate`].
///
/// # Usage
///
/// ```no_run
/// use bevy::prelude::*;
/// use opts_bevy::OptsCoordsPlugin;
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(OptsCoordsPlugin)
///     .run();
/// ```
#[derive(Default)]
pub struct OptsCoordsPlugin;

impl Plugin for OptsCoordsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<OmsiCoords>()
            .register_type::<OmsiTransform>()
            .add_systems(PostUpdate, sync_coordinate_transforms);
    }
}

use crate::scenery::{SceneryObjectAsset, ScoAssetLoader, SliAssetLoader, SplineProfileAsset};

/// Bevy plugin that registers native asset loaders for OMSI scenery objects (`.sco`) and splines (`.sli`).
#[derive(Default)]
pub struct OptsSceneryPlugin;

impl Plugin for OptsSceneryPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<SceneryObjectAsset>()
            .init_asset::<SplineProfileAsset>()
            .init_asset_loader::<ScoAssetLoader>()
            .init_asset_loader::<SliAssetLoader>();
    }
}

pub use crate::vehicle_sim::OptsVehiclePlugin;

/// Unified Bevy plugin that bundles mesh loaders, texture loaders, scenery loaders, coordinate conversion, and vehicle simulation systems.
///
/// Registers `.o3d`, `.x`, `.dds`, `.bmp`, `.tga`, `.sco`, and `.sli` asset loaders, coordinate sync, and vehicle physics with the Bevy `App`.
///
/// # Usage
///
/// ```no_run
/// use bevy::prelude::*;
/// use opts_bevy::OptsAssetPlugin;
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(OptsAssetPlugin)
///     .run();
/// ```
#[derive(Default)]
pub struct OptsAssetPlugin;

impl Plugin for OptsAssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(OptsMeshPlugin)
            .add_plugins(OptsMaterialPlugin)
            .add_plugins(OptsCoordsPlugin)
            .add_plugins(OptsSceneryPlugin)
            .add_plugins(OptsVehiclePlugin)
            .add_plugins(crate::vehicle_script::VehicleScriptPlugin)
            .add_plugins(crate::articulation::ArticulationPlugin)
            .add_plugins(crate::vehicle_anim::VehicleAnimationPlugin)
            .add_plugins(crate::vehicle_lights::VehicleLightPlugin)
            .add_plugins(crate::vehicle_triggers::VehicleTriggerPlugin)
            .add_plugins(crate::camera::OptsCameraPlugin);
    }
}
