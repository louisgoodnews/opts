//! Texture resolution for the model loader: finding a named texture on disk, decoding it once
//! per file, and the alpha-free variant used by blended materials.

use std::path::{Path, PathBuf};

use bevy::asset::Handle;
use bevy::image::Image;

use crate::texture::load_bevy_image;

use super::cache::ContentLoader;
use super::season_refresh;

/// `load_bevy_image` under the `tex.decode` stream-trace span (M-0089).
fn decode_traced(bytes: &[u8]) -> Result<Image, crate::texture::TextureError> {
    let _t = opts_logging::stream_trace::span("tex.decode");
    load_bevy_image(bytes)
}

impl ContentLoader<'_> {
    /// The texture without its alpha channel.
    ///
    /// OMSI uses a texture's alpha only where the material asks for it (`[matl_alpha]`,
    /// `[matl_transmap]`); paint textures such as `GN92_1.tga` carry unrelated data there (here
    /// almost all zeros), which made whole bodies invisible when blended. Images that are already
    /// opaque, or compressed, are returned unchanged.
    pub(super) fn opaque_variant(&mut self, handle: &Handle<Image>) -> Handle<Image> {
        if let Some(h) = self.cache.opaque.get(&handle.id()) {
            return h.clone();
        }
        let out = match self.images.get(handle) {
            Some(img)
                if img.texture_descriptor.format
                    == bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
                    && img
                        .data
                        .as_ref()
                        .is_some_and(|d| d.chunks_exact(4).any(|p| p[3] != 255)) =>
            {
                let mut copy = img.clone();
                if let Some(d) = copy.data.as_mut() {
                    for p in d.chunks_exact_mut(4) {
                        p[3] = 255;
                    }
                }
                self.images.add(copy)
            }
            _ => handle.clone(),
        };
        self.cache.opaque.insert(handle.id(), out.clone());
        out
    }

    pub(super) fn fail(&mut self, what: String) {
        tracing::error!(at = %crate::position_tag::position_tag(), "{what}");
        self.cache.stats.failures.push(what);
    }

    /// The folders searched for a texture under the cover state `snow`: the season variants
    /// (`WinterSnow` under snow cover) first, then the base folders, the global `Texture` folder
    /// and the install root.
    fn search_dirs(&self, dirs: &[PathBuf], snow: bool) -> Vec<PathBuf> {
        let mut search = crate::season::variant_folders(dirs, snow);
        search.extend(dirs.iter().cloned());
        search.push(self.install.root().join("Texture"));
        search.push(self.install.root().to_path_buf());
        search
    }

    /// The file a texture name resolves to under the cover state `snow`. Materials often name a
    /// texture by an extension that does not exist on disk (`textures.md`): the same stem with the
    /// other image extensions is tried next, then a same-named file elsewhere in the install.
    fn find_texture(&self, name: &str, dirs: &[PathBuf], snow: bool) -> Option<PathBuf> {
        let search = self.search_dirs(dirs, snow);
        let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
        let candidates = std::iter::once(name.to_owned()).chain(
            ["dds", "bmp", "tga", "png", "jpg"]
                .iter()
                .map(|e| format!("{stem}.{e}")),
        );
        for cand in candidates {
            for dir in &search {
                if let Some(p) = self.install.find_in(dir, &cand) {
                    return Some(p);
                }
            }
        }
        let near = dirs
            .first()
            .map_or_else(|| self.install.root().to_path_buf(), Clone::clone);
        let hit = self.install.find_by_name(name, &near, true);
        if let Some(p) = &hit {
            tracing::debug!(texture = name, found = %p.display(), "texture rerouted to a same-named file");
        }
        hit
    }

    /// Reads and decodes the file at `path` into the image store, once per path.
    fn decode_texture(&mut self, path: &Path) -> Option<Handle<Image>> {
        let t_read = opts_logging::stream_trace::span("tex.read");
        let read = std::fs::read(path);
        drop(t_read);
        match read {
            Ok(bytes) => match decode_traced(&bytes) {
                Ok(mut image) => {
                    {
                        let _t = opts_logging::stream_trace::span("tex.prepare_mips");
                        crate::texture_filter::prepare_object_texture(&mut image);
                    }
                    self.cache.stats.textures += 1;
                    tracing::debug!(path = %path.display(), "loaded texture");
                    let _t = opts_logging::stream_trace::span("tex.assets_add");
                    Some(self.images.add(image))
                }
                Err(e) => {
                    self.fail(format!("texture decode failed {}: {e}", path.display()));
                    None
                }
            },
            Err(e) => {
                self.fail(format!("texture read failed {}: {e}", path.display()));
                None
            }
        }
    }

    /// Loads a texture, searching `dirs` in order and then the global `Texture` folder. A new
    /// texture is recorded for the snow refresh (`season_refresh`) with its file under both cover
    /// states, so it follows a later change of the snow cover.
    pub fn texture(&mut self, name: &str, dirs: &[PathBuf]) -> Option<Handle<Image>> {
        let name = name.trim();
        // Authors write `------` for "no texture"; that is not a missing file.
        if name.is_empty() || name.chars().all(|c| c == '-') {
            return None;
        }
        let snow = crate::season::snow_cover();
        let Some(path) = self.find_texture(name, dirs, snow) else {
            // Not real files: `\S:n` style names are script/slot references and `Textfeld_*`
            // textures are text fields OMSI renders at run time.
            if name.starts_with('\\') || name.to_lowercase().starts_with("textfeld") {
                tracing::debug!(
                    texture = name,
                    "runtime-generated texture reference (not a file)"
                );
                return None;
            }
            let searched = self.search_dirs(dirs, snow).len();
            self.fail(format!(
                "texture not found: {name} (searched {searched} folders)"
            ));
            return None;
        };
        self.note_moisture(name, &path);
        let handle = match self.cache.textures.get(&path) {
            Some(cached) => cached.clone(),
            None => {
                let loaded = self.decode_texture(&path);
                self.cache.textures.insert(path.clone(), loaded.clone());
                loaded
            }
        };
        if let Some(h) = &handle {
            if !season_refresh::is_recorded(h.id()) {
                let other = self.find_texture(name, dirs, !snow);
                let (snow_path, dry_path) = if snow {
                    (Some(path), other)
                } else {
                    (other, Some(path))
                };
                season_refresh::record(h.id(), snow, snow_path, dry_path);
            }
        }
        handle
    }
}
