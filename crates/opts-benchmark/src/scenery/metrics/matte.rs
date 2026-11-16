//! Two-colour matting: separating the object from its black and white backgrounds.

use super::super::framing::IMAGE_SIZE;
use super::super::raster::Mask;

/// The object as separated from the two backgrounds.
#[derive(Debug, Clone)]
pub struct Matte {
    /// Pixels with coverage `α ≥ 0.5`.
    pub mask: Mask,
    /// Un-premultiplied colour `I_black / α` (black where `α = 0`).
    pub rgb: Vec<[u8; 3]>,
    /// Object pixels in the sentinel colour.
    pub sentinel: Mask,
}

/// Separates the object from the backgrounds: `d = max |I_white − I_black|`, `α = 1 − d / 255`.
#[must_use]
pub fn matte(black: &[u8], white: &[u8]) -> Matte {
    let n = (IMAGE_SIZE * IMAGE_SIZE) as usize;
    let size = IMAGE_SIZE as usize;
    let mut out = Matte {
        mask: Mask::new(size, size),
        rgb: vec![[0; 3]; n],
        sentinel: Mask::new(size, size),
    };
    for (i, (b, w)) in black
        .chunks_exact(4)
        .zip(white.chunks_exact(4))
        .enumerate()
        .take(n)
    {
        let d = (0..3).map(|c| b[c].abs_diff(w[c])).max().unwrap_or(0);
        let alpha = 1.0 - f32::from(d) / 255.0;
        if alpha < 0.5 {
            continue;
        }
        out.mask.px[i] = true;
        let rgb = [0, 1, 2].map(|c| (f32::from(b[c]) / alpha).round().min(255.0) as u8);
        out.rgb[i] = rgb;
        let [r, g, bl] = opts_bevy::texture_sentinel::SENTINEL_RGB;
        if alpha > 0.9
            && rgb[0].abs_diff(r) <= 3
            && rgb[1].abs_diff(g) <= 3
            && rgb[2].abs_diff(bl) <= 3
        {
            out.sentinel.px[i] = true;
        }
    }
    out
}
