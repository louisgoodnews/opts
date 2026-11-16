//! The sky texture blend: the three sky textures weighted by the sun's height, pulled towards the
//! fog colour per row.

use super::super::Sky;

/// The sky texture for the blend weights of the sun's height. `veil` gives, per row (`0` zenith …
/// `1` horizon), how far the row is pulled towards `veil_color` (the fog colour, sRGB `0..=1`);
/// a veil of 0 leaves the texture as it is.
pub(in super::super) fn blended_sky(
    sky: &Sky,
    weights: [f32; 3],
    veil: impl Fn(f32) -> f32,
    veil_color: [f32; 3],
) -> Vec<u8> {
    let (w, h) = (sky.textures[0].width, sky.textures[0].height);
    let mut out = vec![255u8; w * h * 4];
    for y in 0..h {
        let v = veil(y as f32 / (h - 1).max(1) as f32).clamp(0.0, 1.0);
        for x in 0..w {
            for c in 0..3 {
                let mut acc = 0.0;
                for (t, wt) in sky.textures.iter().zip(weights) {
                    if wt > 0.0 {
                        let (tx, ty) = (x * t.width / w, y * t.height / h);
                        acc += f32::from(t.rgba[(ty * t.width + tx) * 4 + c]) * wt;
                    }
                }
                if v > 0.0 {
                    acc += (veil_color[c] * 255.0 - acc) * v;
                }
                out[(y * w + x) * 4 + c] = acc.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}
