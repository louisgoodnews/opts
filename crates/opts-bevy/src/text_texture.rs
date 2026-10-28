//! Text textures: rasterises `[texttexture]` strings with OMSI bitmap fonts (`Fonts/*.oft`).
//!
//! A font is a colour bitmap plus an alpha bitmap; the `.oft` file gives each character's cell.
//! The text is laid out at native pixel size on one line, aligned per `orientation`, and shrunk
//! uniformly when it does not fit. Layout rules and open questions: `docs/src/architecture/text-textures.md`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::asset::{Assets, Handle};
use bevy::image::Image;
#[allow(unused_imports)] // `OftChar` is for the tests of `script_text`
pub(crate) use opts_core::formats::oft::{OftChar, OftFile, OftFont};

use crate::install::OmsiInstall;

mod gpu;
pub use gpu::{led_image, rebind_redrawn_images, text_image, RedrawnImages};

/// What to draw: one `[texttexture]` slot resolved against one object instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextSpec {
    /// Font name from the `.sco`.
    pub font: String,
    /// The resolved string.
    pub text: String,
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
    /// `true`: draw in `rgb`; `false`: keep the font bitmap's own colours.
    pub mono: bool,
    /// Text colour for mono rendering.
    pub rgb: [u8; 3],
    /// 0 centred, 1 left, 2 right.
    pub orientation: u32,
}

/// Decoded font bitmaps.
#[derive(Debug, Clone)]
pub struct FontBitmaps {
    /// Colour bitmap width.
    pub width: u32,
    /// Colour bitmap height.
    pub height: u32,
    /// RGB, row-major from the top.
    pub colour: Vec<[u8; 3]>,
    /// Alpha bitmap size. Usually equal to the colour bitmap, but 50 of the corpus's 1,108 fonts
    /// differ (printer and IBIS fonts: a black 128×128 colour bitmap with a 128×200 or 512×512
    /// alpha bitmap holding the glyphs); the `.oft` cells are pixel positions in both.
    pub alpha_size: (u32, u32),
    /// Coverage (red channel of the alpha bitmap), row-major from the top.
    pub alpha: Vec<u8>,
}

impl FontBitmaps {
    /// Coverage at pixel `x`, `y` of the alpha bitmap (0 outside it). The cell coordinates are
    /// not scaled to the colour bitmap: a 128×200 alpha bitmap read through a 128×128 colour
    /// bitmap's proportions put the `j` into every space of the printer fonts.
    pub(crate) fn coverage(&self, x: u32, y: u32) -> u8 {
        let (aw, ah) = self.alpha_size;
        if x >= aw || y >= ah {
            return 0;
        }
        self.alpha.get((y * aw + x) as usize).copied().unwrap_or(0)
    }

    /// Colour at pixel `x`, `y` of the colour bitmap; black outside it (glyph rows that only the
    /// larger alpha bitmap holds).
    pub(crate) fn colour_at(&self, x: u32, y: u32) -> [u8; 3] {
        if x >= self.width || y >= self.height {
            return [0, 0, 0];
        }
        self.colour
            .get((y * self.width + x) as usize)
            .copied()
            .unwrap_or([0, 0, 0])
    }

    /// Whether `x`, `y` lies in either bitmap (where a glyph can have pixels).
    pub(crate) fn contains(&self, x: u32, y: u32) -> bool {
        (x < self.width && y < self.height) || (x < self.alpha_size.0 && y < self.alpha_size.1)
    }
}

/// Width in pixels of `text` drawn at native size: the glyph cells plus the font's gap between
/// neighbours (characters without a cell are skipped).
#[must_use]
pub fn text_width(font: &OftFont, text: &str) -> u32 {
    let n = text.chars().filter(|c| font.chars.contains_key(c)).count() as u32;
    cells_width(font, text) + font.gap * n.saturating_sub(1)
}

/// The glyph cells of `text` alone, without the gap: what `TextLength` gives scripts. Display
/// scripts size their `STTextOut` spacing from it (VMatrix.osc of the NG272), and their boards
/// come out right only without the gap ❓.
#[must_use]
pub fn cells_width(font: &OftFont, text: &str) -> u32 {
    text.chars()
        .filter_map(|c| font.chars.get(&c))
        .map(|g| g.x_end.saturating_sub(g.x_start))
        .sum()
}

/// Rasterises `spec.text` into an RGBA8 buffer of `spec.width × spec.height` (transparent
/// background).
#[must_use]
pub fn render_rgba(font: &OftFont, bmp: &FontBitmaps, spec: &TextSpec) -> Vec<u8> {
    let (w, h) = (spec.width.max(1), spec.height.max(1));
    let mut out = vec![0u8; (w * h * 4) as usize];
    let natural_w = text_width(font, &spec.text);
    let line_h = font.line_height.max(1);
    if natural_w == 0 {
        return out;
    }
    // Uniform shrink when the line does not fit; never enlarge.
    let scale = (w as f32 / natural_w as f32)
        .min(h as f32 / line_h as f32)
        .min(1.0);
    let drawn_w = natural_w as f32 * scale;
    let drawn_h = line_h as f32 * scale;
    let origin_x = match spec.orientation {
        1 => 0.0,
        2 => w as f32 - drawn_w,
        _ => (w as f32 - drawn_w) / 2.0,
    };
    let origin_y = (h as f32 - drawn_h) / 2.0;
    // Glyph start offsets along the line, in font pixels.
    let mut pen = 0u32;
    let mut cells = Vec::new();
    for c in spec.text.chars() {
        if let Some(g) = font.chars.get(&c) {
            cells.push((pen, *g));
            pen += g.x_end.saturating_sub(g.x_start) + font.gap;
        }
    }
    for dy in 0..h {
        let ty = (dy as f32 + 0.5 - origin_y) / scale;
        if ty < 0.0 || ty >= line_h as f32 {
            continue;
        }
        for dx in 0..w {
            let tx = (dx as f32 + 0.5 - origin_x) / scale;
            if tx < 0.0 || tx >= natural_w as f32 {
                continue;
            }
            let tx = tx as u32;
            // Last cell starting at or before tx.
            let idx = cells
                .partition_point(|(start, _)| *start <= tx)
                .saturating_sub(1);
            let (start, g) = cells[idx];
            if tx - start >= g.x_end.saturating_sub(g.x_start) {
                continue; // the gap after a glyph
            }
            let (sx, sy) = (g.x_start + (tx - start), g.y_top + ty as u32);
            if !bmp.contains(sx, sy) {
                continue;
            }
            let a = bmp.coverage(sx, sy);
            let rgb = if spec.mono {
                spec.rgb
            } else {
                bmp.colour_at(sx, sy)
            };
            let o = ((dy * w + dx) * 4) as usize;
            out[o..o + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], a]);
        }
    }
    out
}

fn load_bitmaps(install: &OmsiInstall, dir: &Path, font: &OftFont) -> Result<FontBitmaps, String> {
    let find = |name: &str| {
        install
            .find_in(dir, name)
            .ok_or_else(|| format!("bitmap `{name}` not found"))
    };
    decode_bitmaps(&find(&font.bitmap)?, &find(&font.alpha)?)
}

/// Decodes a font's colour and alpha bitmap files.
pub(crate) fn decode_bitmaps(colour: &Path, alpha: &Path) -> Result<FontBitmaps, String> {
    let read = |p: &Path| -> Result<image::DynamicImage, String> {
        let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
        image::load_from_memory(&bytes).map_err(|e| format!("{}: {e}", p.display()))
    };
    let colour = read(colour)?.to_rgb8();
    let alpha = read(alpha)?.to_rgb8();
    Ok(FontBitmaps {
        width: colour.width(),
        height: colour.height(),
        colour: colour.pixels().map(|p| p.0).collect(),
        alpha_size: (alpha.width(), alpha.height()),
        alpha: alpha.pixels().map(|p| p.0[0]).collect(),
    })
}

/// Font lookup by name plus a cache of rendered text images.
#[derive(Default)]
pub struct FontRegistry {
    /// Lower-case font name → (`.oft` path, font).
    fonts: Option<HashMap<String, (PathBuf, Arc<OftFont>)>>,
    bitmaps: HashMap<String, Option<Arc<FontBitmaps>>>,
    rendered: HashMap<TextSpec, Handle<Image>>,
    reported: HashSet<String>,
}

impl FontRegistry {
    fn index(&mut self, install: &OmsiInstall) -> &HashMap<String, (PathBuf, Arc<OftFont>)> {
        self.fonts.get_or_insert_with(|| {
            let mut files = Vec::new();
            if let Some(dir) = install.find_file("Fonts") {
                collect_oft(&dir, &mut files);
            }
            files.sort();
            let mut map = HashMap::new();
            for path in files {
                let parsed = std::fs::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|b| OftFile::parse(&b));
                match parsed {
                    // Which of two same-named fonts wins is open (Q-FN01): first in path order.
                    Ok(f) => {
                        for font in f.fonts {
                            map.entry(font.name.to_lowercase())
                                .or_insert_with(|| (path.clone(), Arc::new(font)));
                        }
                    }
                    Err(e) => tracing::error!(at = %crate::position_tag::position_tag(), font = %path.display(), "font parse failed: {e}"),
                }
            }
            tracing::info!(fonts = map.len(), "font registry built");
            map
        })
    }

    fn report_once(&mut self, key: String, message: &str) {
        if self.reported.insert(key.clone()) {
            tracing::error!(at = %crate::position_tag::position_tag(), font = %key, "{message}");
        }
    }

    /// Rasterises `spec` to RGBA8 pixels (`width × height`). `None` when the font or its bitmaps
    /// cannot be loaded; the error is logged once per font.
    pub fn render_data(&mut self, install: &OmsiInstall, spec: &TextSpec) -> Option<Vec<u8>> {
        let key = spec.font.to_lowercase();
        let Some((path, font)) = self.index(install).get(&key).cloned() else {
            self.report_once(key, "font not found in Fonts/");
            return None;
        };
        if !self.bitmaps.contains_key(&key) {
            let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
            let loaded = match load_bitmaps(install, &dir, &font) {
                Ok(b) => Some(Arc::new(b)),
                Err(e) => {
                    self.report_once(key.clone(), &format!("font bitmaps unusable: {e}"));
                    None
                }
            };
            self.bitmaps.insert(key.clone(), loaded);
        }
        let bmp = self.bitmaps.get(&key)?.clone()?;
        Some(render_rgba(&font, &bmp, spec))
    }

    /// Renders `spec` (or returns the cached image). `None` when the font or its bitmaps cannot
    /// be loaded; the error is logged once per font.
    pub fn render(
        &mut self,
        install: &OmsiInstall,
        images: &mut Assets<Image>,
        spec: &TextSpec,
    ) -> Option<Handle<Image>> {
        if let Some(h) = self.rendered.get(spec) {
            return Some(h.clone());
        }
        let data = self.render_data(install, spec)?;
        let handle = images.add(text_image(spec.width, spec.height, data));
        self.rendered.insert(spec.clone(), handle.clone());
        Some(handle)
    }
}

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

fn collect_oft(dir: &Path, out: &mut Vec<PathBuf>) {
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

#[cfg(test)]
mod tests;
