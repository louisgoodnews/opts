//! Text on script textures (`STTextOut`): the glyphs of the OMSI bitmap fonts drawn onto a
//! canvas in the colours of the font's colour bitmap (the matrix fonts carry their LED colour
//! there; the script's `STSetColor` is not used for text, open question E3), with the font's
//! alpha bitmap as coverage.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::host_callbacks::ScriptTextureCanvas;
use crate::install::OmsiInstall;
use crate::text_texture::{decode_bitmaps, load_font_files, FontBitmaps, OftFont};

/// The bitmaps of the fonts of a game session, decoded the first time a font is drawn with.
/// Indexed like the font list the scripts get from `GetFontIndex`.
#[derive(Clone, Default)]
pub struct FontBitmapSource {
    files: Arc<Vec<Option<(PathBuf, PathBuf)>>>,
    cache: Arc<Mutex<HashMap<usize, Option<Arc<FontBitmaps>>>>>,
}

impl FontBitmapSource {
    /// Finds the bitmap files of every font of the installation (nothing is decoded yet).
    #[must_use]
    pub fn new(install: &OmsiInstall) -> Self {
        let files = load_font_files(install)
            .into_iter()
            .map(|(dir, font)| {
                Some((
                    install.find_in(&dir, &font.bitmap)?,
                    install.find_in(&dir, &font.alpha)?,
                ))
            })
            .collect();
        Self {
            files: Arc::new(files),
            cache: Arc::default(),
        }
    }

    /// The bitmaps of font number `index`; `None` when its files are missing or unreadable.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<Arc<FontBitmaps>> {
        let mut cache = self.cache.lock().ok()?;
        cache
            .entry(index)
            .or_insert_with(|| {
                let (colour, alpha) = self.files.get(index)?.as_ref()?;
                match decode_bitmaps(colour, alpha) {
                    Ok(b) => Some(Arc::new(b)),
                    Err(e) => {
                        tracing::error!(at = %crate::position_tag::position_tag(), font = index, "script texture font unusable: {e}");
                        None
                    }
                }
            })
            .clone()
    }
}

/// Draws `text` with its top left corner at (`x`, `y`): every glyph cell of the font's bitmaps is
/// blended onto the canvas in its own colour, scaled by the glyph's coverage; each glyph moves
/// the pen on by its width (cell including its gap) plus `spacing`.
pub fn draw_text(
    canvas: &mut dyn ScriptTextureCanvas,
    x: i32,
    y: i32,
    font: &OftFont,
    bitmaps: &FontBitmaps,
    text: &str,
    spacing: i32,
) {
    let (cw, ch) = (canvas.width() as i32, canvas.height() as i32);
    let line = font.line_height.max(1) as i32;
    let mut pen = x;
    let buffer = canvas.raw_rgba_mut();
    for c in text.chars() {
        let Some(glyph) = font.chars.get(&c) else {
            pen += (line / 2).max(4) + spacing;
            continue;
        };
        let width = glyph.x_end.saturating_sub(glyph.x_start) as i32;
        for gy in 0..line {
            let py = y + gy;
            if py < 0 || py >= ch {
                continue;
            }
            let sy = glyph.y_top + gy as u32;
            for gx in 0..width {
                let px = pen + gx;
                if px < 0 || px >= cw {
                    continue;
                }
                let sx = glyph.x_start + gx as u32;
                if sx >= bitmaps.width || sy >= bitmaps.height {
                    continue;
                }
                let cover = u32::from(bitmaps.coverage(sx, sy));
                if cover == 0 {
                    continue;
                }
                let o = ((py * cw + px) * 4) as usize;
                let dst = &mut buffer[o..o + 4];
                // Source over: the canvas keeps what it had where the glyph is not opaque.
                let inv = 255 - cover;
                let rgb = bitmaps.colour[(sy * bitmaps.width + sx) as usize];
                for (d, s) in dst.iter_mut().zip(rgb) {
                    *d = ((u32::from(s) * cover + u32::from(*d) * inv) / 255) as u8;
                }
                dst[3] = (cover + u32::from(dst[3]) * inv / 255).min(255) as u8;
            }
        }
        pen += width + spacing;
    }
    canvas.mark_dirty();
}

#[cfg(test)]
mod tests;
