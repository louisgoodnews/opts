//! Encoder abstraction for recordings and the PNG-sequence encoder.

use std::path::{Path, PathBuf};

use bevy::prelude::Image;
use serde::Serialize;

use super::error::CaptureError;
use super::storage;

/// One frame as 8-bit RGB, the common input of all encoders.
#[derive(Debug, Clone)]
pub struct RgbFrame {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// `width * height * 3` bytes, row-major.
    pub data: Vec<u8>,
}

impl RgbFrame {
    /// Converts a captured frame (alpha dropped) and scales it by `scale` (0.25..=1.0).
    ///
    /// # Errors
    /// `Failed` when the pixel format cannot be converted.
    pub fn from_image(image: Image, scale: f32) -> Result<Self, CaptureError> {
        let mut rgb = image
            .try_into_dynamic()
            .map_err(|e| CaptureError::Failed(format!("the frame cannot be converted: {e}")))?
            .to_rgb8();
        if scale < 0.999 {
            let w = ((rgb.width() as f32 * scale) as u32).max(2);
            let h = ((rgb.height() as f32 * scale) as u32).max(2);
            rgb = image::imageops::resize(&rgb, w, h, image::imageops::FilterType::Triangle);
        }
        Ok(Self {
            width: rgb.width(),
            height: rgb.height(),
            data: rgb.into_raw(),
        })
    }

    /// The frame resized to exactly `width` x `height` (a window resized mid-recording).
    #[must_use]
    pub fn fitted(self, width: u32, height: u32) -> Self {
        if (self.width, self.height) == (width, height) {
            return self;
        }
        let Some(img) = image::RgbImage::from_raw(self.width, self.height, self.data) else {
            return Self {
                width,
                height,
                data: vec![0; (width * height * 3) as usize],
            };
        };
        let out =
            image::imageops::resize(&img, width, height, image::imageops::FilterType::Triangle);
        Self {
            width,
            height,
            data: out.into_raw(),
        }
    }
}

/// Identity of one recorded frame, written to the manifest or sidecar.
#[derive(Debug, Clone, Serialize)]
pub struct FrameInfo {
    /// 0-based capture index.
    pub index: u64,
    /// Simulation time when the frame was requested.
    pub sim_time: f64,
    /// Render frame counter when the frame was requested.
    pub frame: u64,
    /// Wall-clock milliseconds since the recording started.
    pub wall_ms: u64,
}

/// Consumes frames on the encoder thread.
pub trait Encoder: Send {
    /// Encodes one frame; returns the bytes written so far.
    ///
    /// # Errors
    /// `Failed` on encoding or I/O errors; the recording then stops.
    fn push(&mut self, frame: &RgbFrame, info: &FrameInfo) -> Result<u64, CaptureError>;

    /// Flushes and closes the output; returns the output size in bytes.
    ///
    /// # Errors
    /// `Failed` on I/O errors.
    fn finish(self: Box<Self>) -> Result<u64, CaptureError>;
}

/// Writes `000001.png`, `000002.png`, … into a directory.
pub struct PngSequence {
    dir: PathBuf,
    count: u64,
    bytes: u64,
}

impl PngSequence {
    /// A sequence writing into `dir` (created if missing).
    ///
    /// # Errors
    /// `Failed` when the directory cannot be created.
    pub fn create(dir: &Path) -> Result<Self, CaptureError> {
        std::fs::create_dir_all(dir).map_err(|e| CaptureError::io("create", dir, &e))?;
        Ok(Self {
            dir: dir.to_path_buf(),
            count: 0,
            bytes: 0,
        })
    }
}

impl Encoder for PngSequence {
    fn push(&mut self, frame: &RgbFrame, _info: &FrameInfo) -> Result<u64, CaptureError> {
        let img = image::RgbImage::from_raw(frame.width, frame.height, frame.data.clone())
            .ok_or_else(|| CaptureError::Failed("frame buffer has the wrong size".into()))?;
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .map_err(|e| CaptureError::Failed(format!("PNG encoding failed: {e}")))?;
        self.count += 1;
        let path = self.dir.join(format!("{:06}.png", self.count));
        storage::write_atomic(&path, &out)?;
        self.bytes += out.len() as u64;
        Ok(self.bytes)
    }

    fn finish(self: Box<Self>) -> Result<u64, CaptureError> {
        Ok(self.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(w: u32, h: u32) -> RgbFrame {
        RgbFrame {
            width: w,
            height: h,
            data: (0..w * h * 3).map(|i| (i % 251) as u8).collect(),
        }
    }

    #[test]
    fn fitting_keeps_or_resizes() {
        assert_eq!(frame(8, 6).fitted(8, 6).data.len(), 8 * 6 * 3);
        let f = frame(8, 6).fitted(4, 4);
        assert_eq!((f.width, f.height, f.data.len()), (4, 4, 48));
    }

    #[test]
    fn png_sequence_writes_numbered_files() {
        let dir = std::env::temp_dir().join(format!("opts_pngseq_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut enc = PngSequence::create(&dir).expect("create");
        let info = FrameInfo {
            index: 0,
            sim_time: 0.0,
            frame: 0,
            wall_ms: 0,
        };
        enc.push(&frame(16, 12), &info).expect("push");
        enc.push(&frame(16, 12), &info).expect("push");
        assert_eq!(
            image::open(dir.join("000002.png")).expect("png").width(),
            16
        );
        assert!(Box::new(enc).finish().expect("finish") > 0);
    }
}
