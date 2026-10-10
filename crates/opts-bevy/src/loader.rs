//! Native Bevy [`AssetLoader`] implementations for OMSI `.o3d` and DirectX `.x` mesh formats.

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AsyncReadExt, LoadContext};
use bevy::mesh::Mesh;
use bevy::reflect::TypePath;
use thiserror::Error;

use crate::mesh::{o3d_to_bevy_mesh, x_mesh_to_bevy_meshes};
use opts_core::formats::o3d::{self, O3dParseError};
use opts_core::formats::x::{self, XParseError};

/// Errors that can occur when loading an OMSI `.o3d` mesh asset.
#[derive(Debug, Error)]
pub enum O3dLoaderError {
    /// An I/O error occurred while reading from the asset source.
    #[error("I/O error reading .o3d asset: {0}")]
    Io(#[from] std::io::Error),

    /// Parsing failed within `opts-core`.
    #[error("Failed to parse .o3d binary mesh: {0}")]
    Parse(#[from] O3dParseError),
}

/// Native Bevy [`AssetLoader`] for OMSI `.o3d` 3D mesh files.
#[derive(Default, TypePath)]
pub struct O3dAssetLoader;

impl AssetLoader for O3dAssetLoader {
    type Asset = Mesh;
    type Settings = ();
    type Error = O3dLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let o3d = o3d::parse(&bytes)?;
        Ok(o3d_to_bevy_mesh(&o3d))
    }

    fn extensions(&self) -> &[&str] {
        &["o3d"]
    }
}

/// Errors that can occur when loading a DirectX `.x` mesh asset.
#[derive(Debug, Error)]
pub enum XLoaderError {
    /// An I/O error occurred while reading from the asset source.
    #[error("I/O error reading .x asset: {0}")]
    Io(#[from] std::io::Error),

    /// Parsing failed within `opts-core`.
    #[error("Failed to parse .x mesh: {0}")]
    Parse(#[from] XParseError),

    /// The asset was parsed successfully but contained zero geometry.
    #[error("DirectX .x mesh asset contains no geometry")]
    EmptyMesh,
}

/// Native Bevy [`AssetLoader`] for Microsoft DirectX `.x` text mesh files.
#[derive(Default, TypePath)]
pub struct XAssetLoader;

impl AssetLoader for XAssetLoader {
    type Asset = Mesh;
    type Settings = ();
    type Error = XLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let x_mesh = x::parse_x(&bytes)?;

        let mut all_meshes = x_mesh_to_bevy_meshes(&x_mesh);
        if all_meshes.is_empty() {
            return Err(XLoaderError::EmptyMesh);
        }

        // Register labeled sub-assets for individual sub-meshes if multiple exist
        if all_meshes.len() > 1 {
            for (i, submesh) in all_meshes.iter().enumerate() {
                let label = format!("submesh_{i}");
                load_context.add_labeled_asset(label, submesh.clone());
            }
        }

        // Return combined mesh or the primary mesh as root asset
        if all_meshes.len() == 1 {
            all_meshes.pop().ok_or(XLoaderError::EmptyMesh)
        } else {
            crate::mesh::x_mesh_to_combined_bevy_mesh(&x_mesh).ok_or(XLoaderError::EmptyMesh)
        }
    }

    fn extensions(&self) -> &[&str] {
        &["x"]
    }
}

/// Errors that can occur when loading a texture asset.
#[derive(Debug, Error)]
pub enum TextureLoaderError {
    /// An I/O error occurred while reading from the asset source.
    #[error("I/O error reading texture asset: {0}")]
    Io(#[from] std::io::Error),

    /// Texture decoding failed.
    #[error("Failed to decode texture: {0}")]
    Texture(#[from] crate::texture::TextureError),
}

/// Native Bevy [`AssetLoader`] for DirectDraw Surface (`.dds`) and disguised texture files.
///
/// Uses content sniffing via [`load_bevy_image`](crate::texture::load_bevy_image) to decode
/// `.dds` files regardless of whether they contain true DirectDraw Surface headers or
/// disguised BMP / JPEG / PNG bitmaps.
#[derive(Default, TypePath)]
pub struct DdsAssetLoader;

impl AssetLoader for DdsAssetLoader {
    type Asset = bevy::image::Image;
    type Settings = ();
    type Error = TextureLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let image = crate::texture::load_bevy_image(&bytes)?;
        Ok(image)
    }

    fn extensions(&self) -> &[&str] {
        &["dds"]
    }
}

/// Native Bevy [`AssetLoader`] for legacy OMSI image files (`.bmp`, `.tga`).
#[derive(Default, TypePath)]
pub struct OptsImageLoader;

impl AssetLoader for OptsImageLoader {
    type Asset = bevy::image::Image;
    type Settings = ();
    type Error = TextureLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let image = crate::texture::load_bevy_image(&bytes)?;
        Ok(image)
    }

    fn extensions(&self) -> &[&str] {
        &["bmp", "tga"]
    }
}
