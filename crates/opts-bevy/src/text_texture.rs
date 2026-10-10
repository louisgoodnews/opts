//! Text textures: rasterises `[texttexture]` strings with OMSI bitmap fonts (`Fonts/*.oft`).
//!
//! A font is a colour bitmap plus an alpha bitmap; the `.oft` file gives each character's cell.
//! The text is laid out at native pixel size on one line, aligned per `orientation`, and shrunk
//! uniformly when it does not fit. Layout rules and open questions: `docs/src/architecture/text-textures.md`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::asset::{Assets, Handle};
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use opts_core::formats::oft::{OftFile, OftFont};

use crate::install::OmsiInstall;

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
    /// Alpha bitmap size (normally equal to the colour bitmap).
    pub alpha_size: (u32, u32),
    /// Coverage (red channel of the alpha bitmap), row-major from the top.
    pub alpha: Vec<u8>,
}

impl FontBitmaps {
    fn coverage(&self, x: u32, y: u32) -> u8 {
        let (aw, ah) = self.alpha_size;
        let ax = (x * aw / self.width.max(1)).min(aw.saturating_sub(1));
        let ay = (y * ah / self.height.max(1)).min(ah.saturating_sub(1));
        self.alpha
            .get((ay * aw + ax) as usize)
            .copied()
            .unwrap_or(0)
    }
}

/// Width in pixels of `text` drawn at native size (characters without a cell are skipped).
#[must_use]
pub fn text_width(font: &OftFont, text: &str) -> u32 {
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
            pen += g.x_end.saturating_sub(g.x_start);
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
            let (sx, sy) = (g.x_start + (tx - start), g.y_top + ty as u32);
            if sx >= bmp.width || sy >= bmp.height {
                continue;
            }
            let a = bmp.coverage(sx, sy);
            let rgb = if spec.mono {
                spec.rgb
            } else {
                bmp.colour[(sy * bmp.width + sx) as usize]
            };
            let o = ((dy * w + dx) * 4) as usize;
            out[o..o + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], a]);
        }
    }
    out
}

fn load_bitmaps(install: &OmsiInstall, dir: &Path, font: &OftFont) -> Result<FontBitmaps, String> {
    let read = |name: &str| -> Result<image::DynamicImage, String> {
        let p = install
            .find_in(dir, name)
            .ok_or_else(|| format!("bitmap `{name}` not found"))?;
        let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        image::load_from_memory(&bytes).map_err(|e| format!("{}: {e}", p.display()))
    };
    let colour = read(&font.bitmap)?.to_rgb8();
    let alpha = read(&font.alpha)?.to_rgb8();
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
                    Err(e) => tracing::error!(font = %path.display(), "font parse failed: {e}"),
                }
            }
            tracing::info!(fonts = map.len(), "font registry built");
            map
        })
    }

    fn report_once(&mut self, key: String, message: &str) {
        if self.reported.insert(key.clone()) {
            tracing::error!(font = %key, "{message}");
        }
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
        let data = render_rgba(&font, &bmp, spec);
        let mut image = Image::new(
            Extent3d {
                width: spec.width.max(1),
                height: spec.height.max(1),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        // Mesh UVs may lie outside 0..1 (negative V is common): wrap like every OMSI texture.
        image.sampler = crate::model::repeat_sampler();
        let handle = images.add(image);
        self.rendered.insert(spec.clone(), handle.clone());
        Some(handle)
    }
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
mod tests {
    use super::*;
    use opts_core::formats::oft::OftChar;

    /// Font with two 4-px-wide cells: `A` fully covered, `B` half (left two columns).
    fn demo() -> (OftFont, FontBitmaps) {
        let mut chars = HashMap::new();
        chars.insert(
            'A',
            OftChar {
                x_start: 0,
                x_end: 4,
                y_top: 0,
            },
        );
        chars.insert(
            'B',
            OftChar {
                x_start: 4,
                x_end: 8,
                y_top: 0,
            },
        );
        let font = OftFont {
            name: "Demo".into(),
            bitmap: String::new(),
            alpha: String::new(),
            line_height: 4,
            gap: 0,
            chars,
        };
        let mut alpha = vec![255u8; 32];
        for y in 0..4 {
            alpha[y * 8 + 6] = 0;
            alpha[y * 8 + 7] = 0;
        }
        let bmp = FontBitmaps {
            width: 8,
            height: 4,
            colour: vec![[10, 20, 30]; 32],
            alpha_size: (8, 4),
            alpha,
        };
        (font, bmp)
    }

    fn spec(text: &str, w: u32, h: u32, orientation: u32) -> TextSpec {
        TextSpec {
            font: "Demo".into(),
            text: text.into(),
            width: w,
            height: h,
            mono: true,
            rgb: [200, 0, 0],
            orientation,
        }
    }

    fn alpha_row(buf: &[u8], w: u32, y: u32) -> Vec<u8> {
        (0..w)
            .map(|x| buf[((y * w + x) * 4 + 3) as usize])
            .collect()
    }

    #[test]
    fn width_sums_cells_and_skips_unknown() {
        let (font, _) = demo();
        assert_eq!(text_width(&font, "AB?"), 8);
    }

    #[test]
    fn left_centre_right_alignment() {
        let (font, bmp) = demo();
        let left = render_rgba(&font, &bmp, &spec("A", 12, 4, 1));
        let centre = render_rgba(&font, &bmp, &spec("A", 12, 4, 0));
        let right = render_rgba(&font, &bmp, &spec("A", 12, 4, 2));
        assert_eq!(
            alpha_row(&left, 12, 1),
            [255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            alpha_row(&centre, 12, 1),
            [0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0]
        );
        assert_eq!(
            alpha_row(&right, 12, 1),
            [0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255]
        );
    }

    #[test]
    fn mono_uses_text_colour_and_colour_mode_uses_bitmap() {
        let (font, bmp) = demo();
        let mono = render_rgba(&font, &bmp, &spec("A", 4, 4, 0));
        assert_eq!(&mono[0..4], &[200, 0, 0, 255]);
        let mut s = spec("A", 4, 4, 0);
        s.mono = false;
        let col = render_rgba(&font, &bmp, &s);
        assert_eq!(&col[0..4], &[10, 20, 30, 255]);
    }

    #[test]
    fn alpha_bitmap_cuts_the_glyph() {
        let (font, bmp) = demo();
        let buf = render_rgba(&font, &bmp, &spec("B", 4, 4, 0));
        assert_eq!(alpha_row(&buf, 4, 0), [255, 255, 0, 0]);
    }

    #[test]
    fn long_text_shrinks_to_fit_and_empty_text_is_transparent() {
        let (font, bmp) = demo();
        let buf = render_rgba(&font, &bmp, &spec("AB", 4, 4, 1));
        // Scale 0.5: the 4-px-high line occupies the middle two rows.
        assert_eq!(alpha_row(&buf, 4, 0), [0, 0, 0, 0]);
        assert!(alpha_row(&buf, 4, 1).iter().any(|a| *a > 0));
        assert!(render_rgba(&font, &bmp, &spec("", 4, 4, 0))
            .iter()
            .all(|b| *b == 0));
    }

    #[test]
    fn text_source_by_number_and_by_name() {
        let vars = vec!["Alexanderplatz".to_string(), "5".to_string()];
        let names = vec!["Name".to_string(), "Kind".to_string()];
        assert_eq!(
            resolve_text(0, None, &vars, &names).as_deref(),
            Some("Alexanderplatz")
        );
        assert_eq!(
            resolve_text(0, Some("kind"), &vars, &names).as_deref(),
            Some("5")
        );
        assert_eq!(resolve_text(0, Some("missing"), &vars, &names), None);
        assert_eq!(resolve_text(3, None, &vars, &names), None);
    }
}
