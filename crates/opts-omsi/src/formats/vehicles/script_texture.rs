//! CPU Canvas abstraction and implementation for OMSI script textures (`[scripttexture]`).
//!
//! Provides [`ScriptTextureCanvas`] and [`CpuScriptTexture`] for drawing dashboard displays,
//! matrix destination boards, ticket printer screens and tachographs.
//! Designed to work headlessly without Bevy dependencies so visual script editors
//! and testing harnesses can render and inspect canvases directly.

use std::path::Path;

use crate::formats::oft::OftFont;

/// Trait defining the script texture canvas drawing operations.
pub trait ScriptTextureCanvas: Send + Sync {
    /// Texture width in pixels.
    fn width(&self) -> u32;

    /// Texture height in pixels.
    fn height(&self) -> u32;

    /// Whether the canvas is currently locked by a script.
    fn is_locked(&self) -> bool;

    /// Locks the canvas for drawing operations (`STLock`).
    fn lock(&mut self);

    /// Unlocks the canvas and marks it dirty for texture upload (`STUnlock`).
    fn unlock(&mut self);

    /// Whether the texture content has changed since the last clear of the dirty flag.
    fn is_dirty(&self) -> bool;

    /// Clears the dirty flag once uploaded to GPU.
    fn clear_dirty(&mut self);

    /// Marks the content as changed (drawing done on the raw buffer by the host).
    fn mark_dirty(&mut self);

    /// Sets the active drawing color in ARGB (0-255 per channel).
    fn set_color(&mut self, a: u8, r: u8, g: u8, b: u8);

    /// Gets the current drawing color `[A, R, G, B]`.
    fn current_color(&self) -> [u8; 4];

    /// Fills an axis-aligned rectangle from (x1, y1) to (x2, y2) with the active color (`STDrawRect`).
    fn draw_rect(&mut self, x1: i32, y1: i32, x2: i32, y2: i32);

    /// Draws a single pixel at (x, y) with the active color (`STDrawPixel`).
    fn draw_pixel(&mut self, x: i32, y: i32);

    /// Samples a pixel at (x, y) and stores it for subsequent `STGetR/G/B/A` calls (`STReadPixel`).
    fn read_pixel(&mut self, x: i32, y: i32);

    /// Red channel (0.0..255.0) of the last sampled pixel (`STGetR`).
    fn get_r(&self) -> f32;

    /// Green channel (0.0..255.0) of the last sampled pixel (`STGetG`).
    fn get_g(&self) -> f32;

    /// Blue channel (0.0..255.0) of the last sampled pixel (`STGetB`).
    fn get_b(&self) -> f32;

    /// Alpha channel (0.0..255.0) of the last sampled pixel (`STGetA`).
    fn get_a(&self) -> f32;

    /// Applies a 3x3 smoothing/filter kernel to the canvas (`STFilter`).
    fn filter(&mut self);

    /// Clears the texture to transparent black (`STNewTex`).
    fn new_tex(&mut self);

    /// Loads an image file from disk and blits it onto the canvas (`STLoadTex`).
    fn load_tex(&mut self, path: &Path) -> Result<(), String>;

    /// Decodes image bytes and blits them onto the canvas.
    fn load_tex_bytes(&mut self, bytes: &[u8]) -> Result<(), String>;

    /// Draws a text string onto the canvas using the given font and letter spacing (`STTextOut`).
    fn text_out(
        &mut self,
        x: i32,
        y: i32,
        font: &OftFont,
        text: &str,
        alignment: u32,
        spacing: i32,
    );

    /// Returns the raw RGBA8 pixel buffer (length = `width * height * 4`).
    fn raw_rgba(&self) -> &[u8];

    /// Returns the mutable raw RGBA8 pixel buffer.
    fn raw_rgba_mut(&mut self) -> &mut [u8];
}

/// An in-memory RGBA8 canvas for script textures.
#[derive(Debug, Clone)]
pub struct CpuScriptTexture {
    width: u32,
    height: u32,
    buffer: Vec<u8>,
    locked: bool,
    dirty: bool,
    current_color: [u8; 4], // [A, R, G, B]
    last_read: [u8; 4],     // [A, R, G, B]
}

impl CpuScriptTexture {
    /// Creates a new canvas of the specified width and height initialized to transparent black.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        let w = width.max(1);
        let h = height.max(1);
        let size = (w * h * 4) as usize;
        Self {
            width: w,
            height: h,
            buffer: vec![0u8; size],
            locked: false,
            dirty: true,
            current_color: [255, 255, 255, 255],
            last_read: [0, 0, 0, 0],
        }
    }
}

impl ScriptTextureCanvas for CpuScriptTexture {
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn is_locked(&self) -> bool {
        self.locked
    }

    fn lock(&mut self) {
        self.locked = true;
    }

    fn unlock(&mut self) {
        self.locked = false;
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    fn set_color(&mut self, a: u8, r: u8, g: u8, b: u8) {
        self.current_color = [a, r, g, b];
    }

    fn current_color(&self) -> [u8; 4] {
        self.current_color
    }

    fn draw_rect(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let min_x = x1.min(x2).max(0) as u32;
        let max_x = (x1.max(x2) as u32).min(self.width.saturating_sub(1));
        let min_y = y1.min(y2).max(0) as u32;
        let max_y = (y1.max(y2) as u32).min(self.height.saturating_sub(1));

        if min_x > max_x || min_y > max_y {
            return;
        }

        let [a, r, g, b] = self.current_color;
        let pixel = [r, g, b, a];

        for y in min_y..=max_y {
            let row_offset = (y * self.width) as usize * 4;
            for x in min_x..=max_x {
                let offset = row_offset + (x as usize * 4);
                self.buffer[offset..offset + 4].copy_from_slice(&pixel);
            }
        }
        self.dirty = true;
    }

    fn draw_pixel(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 {
            return;
        }
        let ux = x as u32;
        let uy = y as u32;
        if ux >= self.width || uy >= self.height {
            return;
        }
        let [a, r, g, b] = self.current_color;
        let offset = ((uy * self.width + ux) * 4) as usize;
        self.buffer[offset..offset + 4].copy_from_slice(&[r, g, b, a]);
        self.dirty = true;
    }

    fn read_pixel(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 {
            self.last_read = [0, 0, 0, 0];
            return;
        }
        let ux = x as u32;
        let uy = y as u32;
        if ux >= self.width || uy >= self.height {
            self.last_read = [0, 0, 0, 0];
            return;
        }
        let offset = ((uy * self.width + ux) * 4) as usize;
        let r = self.buffer[offset];
        let g = self.buffer[offset + 1];
        let b = self.buffer[offset + 2];
        let a = self.buffer[offset + 3];
        self.last_read = [a, r, g, b];
    }

    fn get_r(&self) -> f32 {
        self.last_read[1] as f32
    }

    fn get_g(&self) -> f32 {
        self.last_read[2] as f32
    }

    fn get_b(&self) -> f32 {
        self.last_read[3] as f32
    }

    fn get_a(&self) -> f32 {
        self.last_read[0] as f32
    }

    fn filter(&mut self) {
        // 3x3 Box blur to soften matrix display LEDs
        let w = self.width as usize;
        let h = self.height as usize;
        let mut filtered = self.buffer.clone();

        for y in 1..h.saturating_sub(1) {
            for x in 1..w.saturating_sub(1) {
                let mut sum_r = 0u32;
                let mut sum_g = 0u32;
                let mut sum_b = 0u32;
                let mut sum_a = 0u32;

                for dy in [y - 1, y, y + 1] {
                    let row = dy * w * 4;
                    for dx in [x - 1, x, x + 1] {
                        let idx = row + (dx * 4);
                        sum_r += self.buffer[idx] as u32;
                        sum_g += self.buffer[idx + 1] as u32;
                        sum_b += self.buffer[idx + 2] as u32;
                        sum_a += self.buffer[idx + 3] as u32;
                    }
                }

                let dst_idx = (y * w + x) * 4;
                filtered[dst_idx] = (sum_r / 9) as u8;
                filtered[dst_idx + 1] = (sum_g / 9) as u8;
                filtered[dst_idx + 2] = (sum_b / 9) as u8;
                filtered[dst_idx + 3] = (sum_a / 9) as u8;
            }
        }

        self.buffer = filtered;
        self.dirty = true;
    }

    fn new_tex(&mut self) {
        self.buffer.fill(0);
        self.dirty = true;
    }

    fn load_tex(&mut self, path: &Path) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        self.load_tex_bytes(&bytes)
    }

    fn load_tex_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
        let rgba = img.to_rgba8();
        let (src_w, src_h) = (rgba.width(), rgba.height());

        // Blit or scale into canvas
        for y in 0..self.height {
            let sy = (y * src_h / self.height).min(src_h.saturating_sub(1));
            let dst_row = (y * self.width) as usize * 4;
            for x in 0..self.width {
                let sx = (x * src_w / self.width).min(src_w.saturating_sub(1));
                let pixel = rgba.get_pixel(sx, sy);
                let dst_idx = dst_row + (x as usize * 4);
                self.buffer[dst_idx..dst_idx + 4].copy_from_slice(&pixel.0);
            }
        }
        self.dirty = true;
        Ok(())
    }

    fn text_out(
        &mut self,
        x: i32,
        y: i32,
        font: &OftFont,
        text: &str,
        _alignment: u32,
        spacing: i32,
    ) {
        let [a, r, g, b] = self.current_color;
        let pixel = [r, g, b, a];
        let mut pen_x = x;

        for c in text.chars() {
            if let Some(glyph) = font.chars.get(&c) {
                let glyph_w = glyph.x_end.saturating_sub(glyph.x_start);
                let glyph_h = font.line_height.max(1);

                // Draw filled glyph area with current color
                for gy in 0..glyph_h {
                    let py = y + gy as i32;
                    if py < 0 || py >= self.height as i32 {
                        continue;
                    }
                    for gx in 0..glyph_w {
                        let px = pen_x + gx as i32;
                        if px < 0 || px >= self.width as i32 {
                            continue;
                        }
                        let offset = ((py as u32 * self.width + px as u32) * 4) as usize;
                        self.buffer[offset..offset + 4].copy_from_slice(&pixel);
                    }
                }
                pen_x += glyph_w as i32 + spacing + font.gap as i32;
            } else if c == ' ' {
                pen_x += (font.line_height / 2).max(4) as i32 + spacing;
            }
        }
        self.dirty = true;
    }

    fn raw_rgba(&self) -> &[u8] {
        &self.buffer
    }

    fn raw_rgba_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::oft::OftChar;
    use std::collections::HashMap;

    #[test]
    fn cpu_script_texture_draw_rect_and_pixel_sampling() {
        let mut canvas = CpuScriptTexture::new(64, 32);
        assert_eq!(canvas.width(), 64);
        assert_eq!(canvas.height(), 32);

        canvas.lock();
        assert!(canvas.is_locked());

        // Set color A=255, R=200, G=100, B=50
        canvas.set_color(255, 200, 100, 50);
        canvas.draw_rect(10, 5, 20, 15);

        // Sample inside rect
        canvas.read_pixel(15, 10);
        assert_eq!(canvas.get_r(), 200.0);
        assert_eq!(canvas.get_g(), 100.0);
        assert_eq!(canvas.get_b(), 50.0);
        assert_eq!(canvas.get_a(), 255.0);

        // Sample outside rect
        canvas.read_pixel(0, 0);
        assert_eq!(canvas.get_r(), 0.0);
        assert_eq!(canvas.get_g(), 0.0);

        // Sample out of bounds
        canvas.read_pixel(-1, 50);
        assert_eq!(canvas.get_r(), 0.0);

        canvas.unlock();
        assert!(!canvas.is_locked());
        assert!(canvas.is_dirty());
    }

    #[test]
    fn cpu_script_texture_text_out() {
        let mut canvas = CpuScriptTexture::new(128, 32);
        let mut chars = HashMap::new();
        chars.insert(
            'A',
            OftChar {
                x_start: 0,
                x_end: 8,
                y_top: 0,
            },
        );
        chars.insert(
            'B',
            OftChar {
                x_start: 8,
                x_end: 16,
                y_top: 0,
            },
        );

        let font = OftFont {
            name: "TestFont".into(),
            bitmap: "font.bmp".into(),
            alpha: "font_alpha.bmp".into(),
            line_height: 12,
            gap: 1,
            chars,
        };

        canvas.set_color(255, 255, 0, 0);
        canvas.text_out(5, 5, &font, "AB", 0, 2);

        // Pen advances: 5 + 8 (A) + 2 (spacing) + 1 (gap) = 16
        // Character A should be drawn around (5..13, 5..17)
        canvas.read_pixel(6, 6);
        assert_eq!(canvas.get_r(), 255.0);
        assert_eq!(canvas.get_g(), 0.0);
    }
}
