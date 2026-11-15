//! Bevy asset loaders for OMSI `.sco` scenery objects and `.sli` spline profiles.

use bevy::asset::io::Reader;
use bevy::asset::{Asset, AssetLoader, LoadContext};
use bevy::reflect::TypePath;
use opts_core::formats::scenery::{SceneryObject, SceneryParseError, SplineProfile};
use thiserror::Error;

/// Errors that can occur when loading scenery object or spline assets.
#[derive(Debug, Error)]
pub enum SceneryLoaderError {
    /// An I/O error occurred while reading from the asset source.
    #[error("I/O error reading scenery asset: {0}")]
    Io(#[from] std::io::Error),

    /// Parsing failed within `opts-core`.
    #[error("Failed to parse scenery asset: {0}")]
    Parse(#[from] SceneryParseError),
}

/// Bevy [`Asset`] wrapper for a parsed OMSI scenery object definition (`.sco`).
#[derive(Asset, TypePath, Debug, Clone)]
pub struct SceneryObjectAsset {
    /// Parsed scenery object manifest.
    pub object: SceneryObject,
}

/// Bevy [`AssetLoader`] for OMSI `.sco` scenery object files.
#[derive(Default, TypePath)]
pub struct ScoAssetLoader;

impl AssetLoader for ScoAssetLoader {
    type Asset = SceneryObjectAsset;
    type Settings = ();
    type Error = SceneryLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let object = SceneryObject::parse(&bytes)?;
        Ok(SceneryObjectAsset { object })
    }

    fn extensions(&self) -> &[&str] {
        &["sco"]
    }
}

/// Bevy [`Asset`] wrapper for a parsed OMSI spline cross-section profile (`.sli`).
#[derive(Asset, TypePath, Debug, Clone)]
pub struct SplineProfileAsset {
    /// Parsed spline profile.
    pub profile: SplineProfile,
}

/// Bevy [`AssetLoader`] for OMSI `.sli` spline profile files.
#[derive(Default, TypePath)]
pub struct SliAssetLoader;

impl AssetLoader for SliAssetLoader {
    type Asset = SplineProfileAsset;
    type Settings = ();
    type Error = SceneryLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let profile = SplineProfile::parse(&bytes)?;
        Ok(SplineProfileAsset { profile })
    }

    fn extensions(&self) -> &[&str] {
        &["sli"]
    }
}
