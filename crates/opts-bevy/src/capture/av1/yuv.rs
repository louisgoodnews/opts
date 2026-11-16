//! RGB to limited-range BT.709 YUV 4:2:0 conversion for the AV1 encoder.

use super::*;

/// Converts RGB to limited-range BT.709 YUV 4:2:0 planes (Y, U, V).
#[must_use]
pub fn to_yuv420(f: &RgbFrame) -> [Vec<u8>; 3] {
    let (w, h) = (f.width as usize, f.height as usize);
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    let mut y = vec![0u8; w * h];
    let mut u = vec![0u8; cw * ch];
    let mut v = vec![0u8; cw * ch];
    let px = |x: usize, yy: usize| {
        let i = (yy.min(h - 1) * w + x.min(w - 1)) * 3;
        (
            i32::from(f.data[i]),
            i32::from(f.data[i + 1]),
            i32::from(f.data[i + 2]),
        )
    };
    for row in 0..h {
        for col in 0..w {
            let (r, g, b) = px(col, row);
            y[row * w + col] =
                (16 + ((187 * r + 629 * g + 63 * b + 512) >> 10)).clamp(16, 235) as u8;
        }
    }
    for row in 0..ch {
        for col in 0..cw {
            let (mut r, mut g, mut b) = (0, 0, 0);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (pr, pg, pb) = px(col * 2 + dx, row * 2 + dy);
                r += pr;
                g += pg;
                b += pb;
            }
            let (r, g, b) = ((r + 2) >> 2, (g + 2) >> 2, (b + 2) >> 2);
            u[row * cw + col] =
                (128 + ((-103 * r - 347 * g + 450 * b + 512) >> 10)).clamp(16, 240) as u8;
            v[row * cw + col] =
                (128 + ((450 * r - 408 * g - 42 * b + 512) >> 10)).clamp(16, 240) as u8;
        }
    }
    [y, u, v]
}
