//! Scenery textures follow the snow cover when it changes. A texture is decoded once per file, so
//! one loaded before the cover changed kept its variant. The loader records each new texture with
//! its file under both cover states; when the cover changes, the decoded variant is swapped into
//! the same image asset, so every material that uses it follows without being rebuilt.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use bevy::asset::AssetId;
use bevy::image::Image;
use bevy::prelude::*;

use crate::texture::load_bevy_image;

/// One recorded texture: its image asset and the file each cover state shows.
#[derive(Debug)]
struct Entry {
    id: AssetId<Image>,
    /// The file with snow cover (`None`: no such file).
    snow: Option<PathBuf>,
    /// The file without snow cover (`None`: no such file).
    dry: Option<PathBuf>,
    /// Whether the image asset holds the snow file now.
    shows_snow: bool,
}

/// The recorded textures of the loaded scenery.
#[derive(Debug, Default)]
pub struct SeasonTextures {
    entries: Vec<Entry>,
}

impl SeasonTextures {
    /// Whether the image asset is recorded.
    #[must_use]
    pub fn contains(&self, id: AssetId<Image>) -> bool {
        self.entries.iter().any(|e| e.id == id)
    }

    /// Records an image asset that shows `snow` now, with the file of each cover state. An asset
    /// that is already recorded keeps its entry.
    pub fn record(
        &mut self,
        id: AssetId<Image>,
        snow_now: bool,
        snow: Option<PathBuf>,
        dry: Option<PathBuf>,
    ) {
        if !self.contains(id) {
            self.entries.push(Entry {
                id,
                snow,
                dry,
                shows_snow: snow_now,
            });
        }
    }

    /// Swaps the file of cover state `snow` into every recorded image that shows the other state.
    /// A missing file keeps the image it has; an image asset that is gone is dropped. Returns the
    /// number of images swapped.
    pub fn refresh(
        &mut self,
        images: &mut Assets<Image>,
        snow: bool,
        decode: impl Fn(&Path) -> Option<Image>,
    ) -> usize {
        self.entries.retain(|e| images.get(e.id).is_some());
        let mut swapped = 0;
        for e in &mut self.entries {
            if e.shows_snow == snow {
                continue;
            }
            let want = if snow { &e.snow } else { &e.dry };
            let Some(path) = want else { continue };
            let Some(image) = decode(path) else { continue };
            if images.insert(e.id, image).is_err() {
                continue;
            }
            e.shows_snow = snow;
            swapped += 1;
        }
        swapped
    }
}

static TEXTURES: Mutex<SeasonTextures> = Mutex::new(SeasonTextures {
    entries: Vec::new(),
});

/// Records a newly loaded texture (see [`SeasonTextures::record`]).
pub(super) fn record(
    id: AssetId<Image>,
    snow_now: bool,
    snow: Option<PathBuf>,
    dry: Option<PathBuf>,
) {
    if let Ok(mut g) = TEXTURES.lock() {
        g.record(id, snow_now, snow, dry);
    }
}

/// Whether the image asset is recorded for the snow refresh.
pub(super) fn is_recorded(id: AssetId<Image>) -> bool {
    TEXTURES.lock().is_ok_and(|g| g.contains(id))
}

/// Decodes a texture file the way the loader does (mips included).
fn decode_file(path: &Path) -> Option<Image> {
    let bytes = std::fs::read(path).ok()?;
    let mut image = load_bevy_image(&bytes).ok()?;
    crate::texture_filter::prepare_object_texture(&mut image);
    Some(image)
}

/// Brings the recorded scenery textures to cover state `snow`; returns the number swapped.
pub fn refresh_loaded(images: &mut Assets<Image>, snow: bool) -> usize {
    refresh_loaded_with(images, snow, decode_file)
}

/// [`refresh_loaded`] with the decoder given (the tests decode without files).
fn refresh_loaded_with(
    images: &mut Assets<Image>,
    snow: bool,
    decode: impl Fn(&Path) -> Option<Image>,
) -> usize {
    TEXTURES
        .lock()
        .map_or(0, |mut g| g.refresh(images, snow, decode))
}

/// Each frame: follows the snow cover flag (`season::snow_cover`).
pub fn refresh_season_textures(mut images: ResMut<Assets<Image>>) {
    refresh_loaded(&mut images, crate::season::snow_cover());
}

/// Registers the snow refresh of the loaded scenery textures.
pub struct SeasonTexturePlugin;

impl Plugin for SeasonTexturePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            refresh_season_textures.run_if(resource_exists::<Assets<Image>>),
        );
    }
}

#[cfg(test)]
mod tests;
