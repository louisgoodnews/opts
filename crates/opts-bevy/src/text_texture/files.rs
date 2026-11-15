//! The font files of the install and the choice of the string a texture shows.

use super::*;

/// Loads all font definitions found in `Fonts/*.oft`.
#[must_use]
pub fn load_fonts(install: &OmsiInstall) -> Vec<OftFont> {
    load_font_files(install)
        .into_iter()
        .map(|(_, font)| font)
        .collect()
}

/// The fonts of `Fonts/*.oft` in the order [`load_fonts`] lists them, each with the directory of
/// its `.oft` (its bitmaps lie there).
#[must_use]
pub fn load_font_files(install: &OmsiInstall) -> Vec<(PathBuf, OftFont)> {
    let mut files = Vec::new();
    if let Some(dir) = install.find_file("Fonts") {
        collect_oft(&dir, &mut files);
    }
    files.sort();
    let mut fonts = Vec::new();
    for path in files {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(file) = OftFile::parse(&bytes) {
                let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
                fonts.extend(file.fonts.into_iter().map(|f| (dir.clone(), f)));
            }
        }
    }
    fonts
}

pub(super) fn collect_oft(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_oft(&p, out);
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("oft")) {
            out.push(p);
        }
    }
}

/// Picks the string a `[texttexture]` shows: by number into the instance's string variables, or
/// by name through the object's `[stringvarnamelist]` names.
#[must_use]
pub fn resolve_text(
    index: usize,
    name: Option<&str>,
    string_vars: &[String],
    var_names: &[String],
) -> Option<String> {
    let i = match name {
        Some(n) => var_names.iter().position(|v| v.eq_ignore_ascii_case(n))?,
        None => index,
    };
    string_vars.get(i).cloned()
}
