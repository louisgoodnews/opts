//! `[moisture]` of a texture's property file (`<texture>.cfg`, see `texture-cfg.md`) for the
//! model loader: which material names take the global wetness. Dry when the flag or the file is
//! missing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::texture::TextureFlags;

use super::cache::ContentLoader;

/// The cache key of a texture name: trimmed and lower case.
fn key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Records the flag of texture `name` from its property file text (`None`: no file, dry). The
/// first record of a name wins, so a later file with the same name does not change it.
pub(crate) fn record_moisture(map: &mut HashMap<String, bool>, name: &str, cfg: Option<&str>) {
    map.entry(key(name))
        .or_insert_with(|| cfg.is_some_and(|t| TextureFlags::parse(t).moisture));
}

/// Whether a material's texture (by its name) takes the wetness according to `map`.
#[must_use]
pub fn takes_wetness(map: &HashMap<String, bool>, tex_name: Option<&str>) -> bool {
    tex_name
        .and_then(|n| map.get(&key(n)))
        .copied()
        .unwrap_or(false)
}

impl ContentLoader<'_> {
    /// Notes the `[moisture]` flag of the texture `name` found at `path`, once per name.
    pub(super) fn note_moisture(&mut self, name: &str, path: &Path) {
        if !self.cache.moisture.contains_key(&key(name)) {
            let cfg = std::fs::read_to_string(cfg_path(path)).ok();
            record_moisture(&mut self.cache.moisture, name, cfg.as_deref());
        }
    }

    /// Whether the material's texture takes the global wetness (`[moisture]`).
    #[must_use]
    pub fn texture_takes_wetness(&self, tex_name: Option<&str>) -> bool {
        takes_wetness(&self.cache.moisture, tex_name)
    }
}

/// The property file of `texture`: `<texture>.cfg`, next to the image.
fn cfg_path(texture: &Path) -> PathBuf {
    let mut cfg = texture.as_os_str().to_owned();
    cfg.push(".cfg");
    PathBuf::from(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moisture_file_marks_its_name_and_a_missing_one_stays_dry() {
        let mut map = HashMap::new();
        record_moisture(
            &mut map,
            "Str_Asphdrk.bmp",
            Some("[moisture]\n[surface]\n6\n"),
        );
        record_moisture(&mut map, "gras1.dds", None);
        record_moisture(&mut map, "plain.bmp", Some("[puddles]\n"));
        assert!(takes_wetness(&map, Some("str_asphdrk.bmp")));
        assert!(takes_wetness(&map, Some(" STR_ASPHDRK.BMP ")));
        assert!(!takes_wetness(&map, Some("gras1.dds")));
        assert!(!takes_wetness(&map, Some("plain.bmp")));
        assert!(!takes_wetness(&map, Some("unknown.bmp")));
        assert!(!takes_wetness(&map, None));
    }

    #[test]
    fn the_first_record_of_a_name_wins() {
        let mut map = HashMap::new();
        record_moisture(&mut map, "a.bmp", Some("[moisture]\n"));
        record_moisture(&mut map, "a.bmp", Some("[puddles]\n"));
        assert!(takes_wetness(&map, Some("a.bmp")));
    }

    #[test]
    fn the_property_file_is_named_after_the_image() {
        assert_eq!(
            cfg_path(Path::new("/x/Texture/str_asphdrk.bmp")),
            Path::new("/x/Texture/str_asphdrk.bmp.cfg")
        );
    }
}
