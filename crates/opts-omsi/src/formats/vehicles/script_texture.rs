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

    /// `STFilter`: marks the canvas for upload and leaves its pixels unchanged
    /// (OMSI semantics ❓, assumed to be a sampling-mode flag).
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
        // Pixels stay as drawn. Read as a blur, the call would compound across
        // the progressive refresh of VMatrix.osc and smear every board, which
        // OMSI's do not; the design assumes a sampling-mode flag instead, and
        // the LED canvases already sample linearly.
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

        // The bitmap goes to the canvas's top left corner at its own size, clipped: the scripts
        // read single pixels at fixed places back (`STReadPixel 251 3`), so it must not scale.
        for y in 0..self.height.min(src_h) {
            let dst_row = (y * self.width) as usize * 4;
            for x in 0..self.width.min(src_w) {
                let pixel = rgba.get_pixel(x, y);
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
mod tests;
