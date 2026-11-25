//! Block-compressed (BC1 to BC3) texture decoding and the RGB565 colour unpacking for ground maps.

use super::*;

pub(super) fn rgb565(v: u16) -> [u8; 3] {
    let (r, g, b) = ((v >> 11) & 31, (v >> 5) & 63, v & 31);
    [
        ((r * 255 + 15) / 31) as u8,
        ((g * 255 + 31) / 63) as u8,
        ((b * 255 + 15) / 31) as u8,
    ]
}

pub(super) fn decode_bc_color(block: &[u8], opaque_black: bool) -> [[u8; 4]; 4] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let (a, b) = (rgb565(c0), rgb565(c1));
    let mix = |x: u8, y: u8, wx: u32, wy: u32, d: u32| {
        ((u32::from(x) * wx + u32::from(y) * wy) / d) as u8
    };
    let mut p = [[0u8; 4]; 4];
    p[0] = [a[0], a[1], a[2], 255];
    p[1] = [b[0], b[1], b[2], 255];
    if c0 > c1 || !opaque_black {
        p[2] = [
            mix(a[0], b[0], 2, 1, 3),
            mix(a[1], b[1], 2, 1, 3),
            mix(a[2], b[2], 2, 1, 3),
            255,
        ];
        p[3] = [
            mix(a[0], b[0], 1, 2, 3),
            mix(a[1], b[1], 1, 2, 3),
            mix(a[2], b[2], 1, 2, 3),
            255,
        ];
    } else {
        p[2] = [
            mix(a[0], b[0], 1, 1, 2),
            mix(a[1], b[1], 1, 1, 2),
            mix(a[2], b[2], 1, 1, 2),
            255,
        ];
        p[3] = [0, 0, 0, 0];
    }
    p
}

pub(super) fn decode_bc(data: &[u8], w: usize, h: usize, kind: u8) -> Option<Vec<u8>> {
    let block_bytes = if kind == 1 { 8 } else { 16 };
    let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
    if data.len() < bw * bh * block_bytes {
        return None;
    }
    let mut out = vec![0u8; w * h * 4];
    for by in 0..bh {
        for bx in 0..bw {
            let blk = &data[(by * bw + bx) * block_bytes..][..block_bytes];
            let (color_blk, alpha): (&[u8], [u8; 16]) = match kind {
                1 => (blk, [255; 16]),
                2 => {
                    let mut a = [0u8; 16];
                    for i in 0..16 {
                        let nib = (blk[i / 2] >> ((i % 2) * 4)) & 15;
                        a[i] = nib * 17;
                    }
                    (&blk[8..], a)
                }
                _ => {
                    let (a0, a1) = (u32::from(blk[0]), u32::from(blk[1]));
                    let mut table = [0u32; 8];
                    table[0] = a0;
                    table[1] = a1;
                    for i in 2..8u32 {
                        table[i as usize] = if a0 > a1 {
                            ((8 - i) * a0 + (i - 1) * a1) / 7
                        } else if i < 6 {
                            ((6 - i) * a0 + (i - 1) * a1) / 5
                        } else if i == 6 {
                            0
                        } else {
                            255
                        };
                    }
                    let bits =
                        u64::from_le_bytes([blk[2], blk[3], blk[4], blk[5], blk[6], blk[7], 0, 0]);
                    let mut a = [0u8; 16];
                    for (i, v) in a.iter_mut().enumerate() {
                        *v = table[((bits >> (3 * i)) & 7) as usize] as u8;
                    }
                    (&blk[8..], a)
                }
            };
            let pal = decode_bc_color(color_blk, kind == 1);
            let idx = u32::from_le_bytes([color_blk[4], color_blk[5], color_blk[6], color_blk[7]]);
            for py in 0..4 {
                for px in 0..4 {
                    let (x, y) = (bx * 4 + px, by * 4 + py);
                    if x >= w || y >= h {
                        continue;
                    }
                    let i = py * 4 + px;
                    let mut c = pal[((idx >> (2 * i)) & 3) as usize];
                    if kind != 1 {
                        c[3] = alpha[i];
                    }
                    out[(y * w + x) * 4..][..4].copy_from_slice(&c);
                }
            }
        }
    }
    Some(out)
}
