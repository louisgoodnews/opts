//! The decoded font bitmaps and their loading from the colour and alpha files.

use super::*;

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

pub(super) fn load_bitmaps(install: &OmsiInstall, dir: &Path, font: &OftFont) -> Result<FontBitmaps, String> {
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
