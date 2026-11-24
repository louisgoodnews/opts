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

mod cpu;
pub use cpu::CpuScriptTexture;

#[cfg(test)]
mod tests;
