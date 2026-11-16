//! The font folder: the font files read once at start-up and registered as font assets.

use std::path::{Path, PathBuf};

use bevy::prelude::*;

use super::fonts_dir;

const MAX_FILES: usize = 32;
const MAX_FILE_BYTES: u64 = 25 * 1024 * 1024;

/// The font files of `dir` (`*.ttf`, `*.otf`, sorted by name): file name and bytes. Files over
/// 25 MB and everything beyond the first 32 files are left out.
#[must_use]
pub fn read_font_files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x.eq_ignore_ascii_case("ttf") || x.eq_ignore_ascii_case("otf"))
        })
        .collect();
    names.sort();
    let mut out = Vec::new();
    for path in names.into_iter().take(MAX_FILES) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match std::fs::metadata(&path) {
            Ok(m) if m.len() <= MAX_FILE_BYTES => match std::fs::read(&path) {
                Ok(bytes) => out.push((name, bytes)),
                Err(e) => tracing::warn!(%name, "font file cannot be read: {e}"),
            },
            Ok(_) => tracing::warn!(%name, "font file is over 25 MB; skipped"),
            Err(e) => tracing::warn!(%name, "font file cannot be read: {e}"),
        }
    }
    out
}

/// Reads the font folder once and registers every file as a font asset.
pub(super) fn load_user_fonts(fonts: Option<ResMut<Assets<Font>>>) {
    let (Some(mut fonts), Some(dir)) = (fonts, fonts_dir()) else {
        return;
    };
    let files = read_font_files(&dir);
    if !files.is_empty() {
        tracing::info!(count = files.len(), dir = %dir.display(), "user fonts");
    }
    for (_, bytes) in files {
        fonts.add(Font::from_bytes(bytes));
    }
}
