//! sRGB transfer functions as lookup tables: the decode to linear light and the encode back.

use super::*;

pub(super) fn srgb_to_linear_lut() -> &'static [f32; 256] {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut lut = [0.0; 256];
        for (i, v) in lut.iter_mut().enumerate() {
            let c = i as f32 / 255.0;
            *v = if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            };
        }
        lut
    })
}

pub(super) const LINEAR_STEPS: usize = 4096;

pub(super) fn linear_to_srgb_lut() -> &'static [u8; LINEAR_STEPS + 1] {
    static LUT: OnceLock<[u8; LINEAR_STEPS + 1]> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut lut = [0u8; LINEAR_STEPS + 1];
        for (i, v) in lut.iter_mut().enumerate() {
            let l = i as f32 / LINEAR_STEPS as f32;
            let c = if l <= 0.003_130_8 {
                l * 12.92
            } else {
                1.055 * l.powf(1.0 / 2.4) - 0.055
            };
            *v = (c * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        }
        lut
    })
}

pub(super) fn encode_srgb(linear: f32) -> u8 {
    let i = (linear.clamp(0.0, 1.0) * LINEAR_STEPS as f32 + 0.5) as usize;
    linear_to_srgb_lut()[i.min(LINEAR_STEPS)]
}

pub(super) const ENC_STEPS: usize = 16384;

/// sRGB byte -> linear light as 16-bit integer (for the integer fast path).
pub(super) fn srgb_to_linear16() -> &'static [u16; 256] {
    static LUT: OnceLock<[u16; 256]> = OnceLock::new();
    LUT.get_or_init(|| {
        let f = srgb_to_linear_lut();
        let mut lut = [0u16; 256];
        for (o, v) in lut.iter_mut().zip(f) {
            *o = (v * 65535.0 + 0.5) as u16;
        }
        lut
    })
}

/// 14-bit linear light -> sRGB byte.
pub(super) fn linear14_to_srgb() -> &'static [u8; ENC_STEPS + 1] {
    static LUT: OnceLock<[u8; ENC_STEPS + 1]> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut lut = [0u8; ENC_STEPS + 1];
        for (i, v) in lut.iter_mut().enumerate() {
            let l = i as f32 / ENC_STEPS as f32;
            let c = if l <= 0.003_130_8 {
                l * 12.92
            } else {
                1.055 * l.powf(1.0 / 2.4) - 0.055
            };
            *v = (c * 255.0 + 0.5).clamp(0.0, 255.0) as u8;
        }
        lut
    })
}
