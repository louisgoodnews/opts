//! Bevy plugin registering OMSI mesh asset loaders.

use bevy::app::{App, Plugin};
use bevy::asset::AssetApp;

use crate::loader::{O3dAssetLoader, XAssetLoader};

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
        app.init_asset_loader::<crate::loader::DdsAssetLoader>()
            .init_asset_loader::<crate::loader::OptsImageLoader>();
    }
}

/// Unified Bevy plugin that bundles both mesh and texture asset loaders.
///
/// Registers `.o3d`, `.x`, `.dds`, `.bmp`, and `.tga` asset loaders with the Bevy `App`.
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
            .add_plugins(OptsMaterialPlugin);
    }
}
