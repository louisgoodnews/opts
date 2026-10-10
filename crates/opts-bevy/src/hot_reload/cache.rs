//! Cache invalidation used when a file changes (`ContentCache` lives in `model.rs`).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use bevy::asset::Assets;
use bevy::render::texture::Image;

use crate::model::{repeat_sampler, ContentCache};

/// A path with symlinks and `..` resolved, so paths from the watcher and from the loaders compare equal.
#[must_use]
pub fn normalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

impl ContentCache {
    /// Forgets every mesh built from `path` (and its "scrambled" mark); returns how many were dropped.
    pub fn invalidate_mesh(&mut self, path: &Path) -> usize {
        let target = normalize(path);
        let before = self.parts.len();
        self.parts.retain(|(p, _), _| normalize(p) != target);
        self.protected.retain(|p| normalize(p) != target);
        before - self.parts.len()
    }

    /// Number of cached meshes.
    #[must_use]
    pub fn mesh_count(&self) -> usize {
        self.parts.len()
    }

    /// Forgets all meshes (scene reload).
    pub fn clear_meshes(&mut self) {
        self.parts.clear();
        self.protected.clear();
    }

    /// Decodes the texture at `path` again and overwrites the pixels of its existing image, so every
    /// material that uses it updates without being rebuilt. `Ok(false)` when it was never loaded.
    ///
    /// # Errors
    /// A message when the file cannot be read or decoded (the old image stays).
    pub fn reload_texture(&self, path: &Path, images: &mut Assets<Image>) -> Result<bool, String> {
        let target = normalize(path);
        let Some(handle) = self
            .textures
            .iter()
            .find(|(p, _)| normalize(p) == target)
            .and_then(|(_, h)| h.clone())
        else {
            return Ok(false);
        };
        let bytes =
            std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let mut image = crate::texture::load_bevy_image(&bytes).map_err(|e| e.to_string())?;
        image.sampler = repeat_sampler();
        match images.get_mut(&handle) {
            Some(slot) => {
                *slot = image;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Directories of every file this cache has loaded (meshes and textures).
    #[must_use]
    pub fn loaded_dirs(&self) -> HashSet<PathBuf> {
        self.parts
            .keys()
            .map(|(p, _)| p)
            .chain(self.textures.keys())
            .filter_map(|p| p.parent().map(Path::to_path_buf))
            .collect()
    }
}
