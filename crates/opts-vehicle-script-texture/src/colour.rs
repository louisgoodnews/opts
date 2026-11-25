//! The colour textures of script textures: a canvas that takes its colours from a picture (a
//! screen) or a single bright colour (an LED's texture).

use bevy::prelude::*;

/// A decoded colour texture (RGBA8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColourMap {
    /// Width (px).
    pub width: u32,
    /// Height (px).
    pub height: u32,
    /// Pixels, row by row.
    pub rgba: Vec<u8>,
}

impl ColourMap {
    /// Whether the map covers a canvas of `width × height` as a picture of the same proportions
    /// (not a small swatch, such as an LED's texture).
    #[must_use]
    pub fn fits(&self, width: u32, height: u32) -> bool {
        self.width * 2 >= width
            && self.height * 2 >= height
            && (u64::from(self.width) * u64::from(height))
                .abs_diff(u64::from(self.height) * u64::from(width))
                <= u64::from(width.max(height))
    }

    /// Puts the map's colour under every pixel of a canvas of `width × height` (nearest pixel),
    /// keeping the canvas's alpha.
    pub fn paint(&self, data: &mut [u8], width: u32, height: u32) {
        for (i, p) in data.chunks_exact_mut(4).enumerate() {
            let (x, y) = (i as u32 % width, i as u32 / width);
            let (mx, my) = (x * self.width / width, y * self.height / height);
            let o = ((my * self.width + mx) * 4) as usize;
            if let Some(c) = self.rgba.get(o..o + 3) {
                p[..3].copy_from_slice(c);
            }
        }
    }
}

/// Where a tinted canvas takes its colour texture's name from.
#[derive(Debug, Clone)]
pub struct ColourSource {
    /// The entity whose engine owns the variable.
    pub owner: Entity,
    /// The string variable.
    pub variable: String,
    /// The folders to look for the texture in.
    pub dirs: Vec<std::path::PathBuf>,
    /// The name last looked at.
    pub last: String,
}

/// A colour texture's pixels.
pub(super) fn colour_map(image: &Image) -> Option<ColourMap> {
    let rgba = opts_bevy::texture::decode_to_rgba8(image)?;
    let (width, height) = (rgba.width(), rgba.height());
    let data = rgba.data?;
    Some(ColourMap {
        width,
        height,
        rgba: data,
    })
}

/// The colour of a light-emitting texture (an LED's `.dds`): the average of its bright pixels.
#[must_use]
pub fn bright_colour(image: &Image) -> Option<[u8; 3]> {
    let rgba = opts_bevy::texture::decode_to_rgba8(image)?;
    let data = rgba.data.as_deref()?;
    let (mut sum, mut n) = ([0u64; 3], 0u64);
    for p in data
        .chunks_exact(4)
        .take(rgba.width() as usize * rgba.height() as usize)
    {
        if p[..3].iter().copied().max().unwrap_or(0) > 96 {
            for i in 0..3 {
                sum[i] += u64::from(p[i]);
            }
            n += 1;
        }
    }
    (n > 0).then(|| [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8])
}
