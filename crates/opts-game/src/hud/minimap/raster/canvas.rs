//! The pixel buffer of the mini map: RGBA8, row 0 on top, with the anti-aliased blend.

/// The buffer: `size * size` RGBA8 pixels, row 0 on top.
#[derive(Debug, Clone)]
pub struct Canvas {
    /// Side in pixels.
    pub size: u32,
    /// The pixels.
    pub data: Vec<u8>,
}

impl Canvas {
    /// A canvas filled with `ground`.
    #[must_use]
    pub fn new(size: u32, ground: [u8; 4]) -> Self {
        let mut data = Vec::with_capacity((size * size * 4) as usize);
        for _ in 0..size * size {
            data.extend_from_slice(&ground);
        }
        Self { size, data }
    }

    /// Resets to `ground` (keeps the allocation).
    pub fn clear(&mut self, ground: [u8; 4]) {
        for px in self.data.chunks_exact_mut(4) {
            px.copy_from_slice(&ground);
        }
    }

    /// The pixel at (`x`, `y`).
    #[must_use]
    pub fn get(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.size + x) * 4) as usize;
        [
            self.data[i],
            self.data[i + 1],
            self.data[i + 2],
            self.data[i + 3],
        ]
    }

    #[inline(always)]
    pub(super) fn blend(&mut self, x: i32, y: i32, color: [u8; 4], coverage: f32) {
        if x < 0 || y < 0 || x >= self.size as i32 || y >= self.size as i32 || coverage <= 0.0 {
            return;
        }
        let i = ((y as u32 * self.size + x as u32) * 4) as usize;
        if coverage >= 1.0 {
            self.data[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], 255]);
            return;
        }
        let c = coverage;
        for (dst, src) in self.data[i..i + 3].iter_mut().zip(color) {
            let old = f32::from(*dst);
            *dst = (old + (f32::from(src) - old) * c + 0.5) as u8;
        }
        self.data[i + 3] = 255;
    }
}
