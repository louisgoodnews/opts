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
