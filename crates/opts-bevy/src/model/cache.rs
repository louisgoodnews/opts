//! Session caches and the borrowed loader view that ties an installation and the Bevy asset
//! stores together for one loading pass.

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::asset::{Assets, Handle};
use bevy::image::Image;
use bevy::mesh::Mesh;
use bevy::pbr::StandardMaterial;

use super::BaseSurface;

use crate::install::OmsiInstall;

use super::types::{ContentStats, ModelPart};

/// A composition: base image, transparency map, and whether the glass pass is split off.
pub(crate) type ComposedKey = (
    bevy::asset::AssetId<Image>,
    bevy::asset::AssetId<Image>,
    bool,
);
/// Its result: the composed picture and the glass pass.
pub(crate) type ComposedImages = (Handle<Image>, Option<Handle<Image>>);

/// Caches shared across a loading session.
#[derive(Default)]
pub struct ContentCache {
    pub(crate) textures: HashMap<PathBuf, Option<Handle<Image>>>,
    /// Textures without their alpha channel, by source image (see [`ContentLoader::opaque_variant`]).
    pub(crate) opaque: HashMap<bevy::asset::AssetId<Image>, Handle<Image>>,
    /// Base texture composed with its transparency map (`[matl_transmap]`): the composed
    /// picture, and its glass pass when the composition is split. The composition decodes both
    /// images and builds a new mip chain (tens of milliseconds); every vehicle of a model family
    /// uses the same pairs, so it is done once.
    pub(crate) composed: HashMap<ComposedKey, ComposedImages>,
    /// A material's texture lit by its light map (`[matl_lightmap]`): the emissive picture, by
    /// (texture, light map). Built once per pair like the transmap composition.
    pub(crate) lit:
        HashMap<(bevy::asset::AssetId<Image>, bevy::asset::AssetId<Image>), Handle<Image>>,
    /// The glass pass of a blended body texture, by source image (`None`: not mostly opaque).
    pub(crate) glass_split: HashMap<bevy::asset::AssetId<Image>, Option<Handle<Image>>>,
    pub(crate) parts: HashMap<(PathBuf, String), Vec<ModelPart>>,
    /// `[crossing_heightdeformation]` base surfaces by file.
    pub(crate) bases: HashMap<PathBuf, Option<std::sync::Arc<BaseSurface>>>,
    /// Shared handles by content (`OPTS_INSTANCING=dedupe`, see [`crate::instancing`]).
    pub pool: crate::instancing::AssetPool,
    /// Static parts of the tile being loaded (`OPTS_INSTANCING=merge`).
    pub merger: crate::instancing::StaticMerger,
    pub(crate) protected: std::collections::HashSet<PathBuf>,
    /// Repaint: texture file name (lower case) → replacement file, applied to the meshes loaded
    /// next (see `docs/src/formats/text/repaints.md`).
    pub texture_replacements: HashMap<String, PathBuf>,
    /// `[moisture]` of each texture name (lower case) from its property file (see
    /// [`super::moisture`]).
    pub(crate) moisture: HashMap<String, bool>,
    /// Set while a vehicle's parts load: their tyre textures take the tyre share of the wetness
    /// (see [`crate::reflections::is_tyre_texture`]). Scenery leaves it off.
    pub vehicle_part: bool,
    /// Loading statistics.
    pub stats: ContentStats,
}

/// Borrowed view tying an installation, caches and Bevy asset stores together.
pub struct ContentLoader<'a> {
    /// Installation used to resolve paths.
    pub install: &'a OmsiInstall,
    /// Session caches and statistics.
    pub cache: &'a mut ContentCache,
    /// Mesh store.
    pub meshes: &'a mut Assets<Mesh>,
    /// Material store.
    pub materials: &'a mut Assets<StandardMaterial>,
    /// Image store.
    pub images: &'a mut Assets<Image>,
}
