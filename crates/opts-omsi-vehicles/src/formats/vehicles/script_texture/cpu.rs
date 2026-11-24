//! The in-memory RGBA8 canvas: the struct, its constructor and the trait implementation.

use std::path::Path;

use crate::formats::oft::OftFont;

use super::ScriptTextureCanvas;

mod draw;
mod text;

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
        self.draw_rect_inner(x1, y1, x2, y2);
    }

    fn draw_pixel(&mut self, x: i32, y: i32) {
        self.draw_pixel_inner(x, y);
    }

    fn read_pixel(&mut self, x: i32, y: i32) {
        self.read_pixel_inner(x, y);
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
        self.text_out_inner(x, y, font, text, _alignment, spacing);
    }

    fn raw_rgba(&self) -> &[u8] {
        &self.buffer
    }

    fn raw_rgba_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }
}
