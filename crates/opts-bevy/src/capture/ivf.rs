//! A minimal IVF writer (the container FFmpeg, mpv and VLC read for raw AV1 streams).
//!
//! Layout: 32-byte file header (`DKIF`, version, header size, FourCC, width, height, time base,
//! frame count), then per frame a 12-byte header (size, presentation timestamp) and the payload.
//! The frame count in the header is patched when the file is finished; a file cut short earlier
//! still plays because players read frames until the end of the file.

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use super::error::CaptureError;

/// Writes an IVF file frame by frame.
pub struct IvfWriter {
    out: BufWriter<File>,
    frames: u32,
    bytes: u64,
}

impl IvfWriter {
    /// Creates `path` and writes the file header.
    ///
    /// # Errors
    /// `Failed` on I/O errors.
    pub fn create(
        path: &Path,
        fourcc: &[u8; 4],
        width: u16,
        height: u16,
        fps: u32,
    ) -> Result<Self, CaptureError> {
        let file = File::create(path).map_err(|e| CaptureError::io("create", path, &e))?;
        let mut out = BufWriter::new(file);
        let mut header = Vec::with_capacity(32);
        header.extend_from_slice(b"DKIF");
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&32u16.to_le_bytes());
        header.extend_from_slice(fourcc);
        header.extend_from_slice(&width.to_le_bytes());
        header.extend_from_slice(&height.to_le_bytes());
        header.extend_from_slice(&fps.to_le_bytes());
        header.extend_from_slice(&1u32.to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes());
        out.write_all(&header)
            .map_err(|e| CaptureError::io("write", path, &e))?;
        Ok(Self {
            out,
            frames: 0,
            bytes: 32,
        })
    }

    /// Appends one frame payload with presentation timestamp `pts` (in time-base units).
    ///
    /// # Errors
    /// `Failed` on I/O errors.
    pub fn write_frame(&mut self, pts: u64, data: &[u8]) -> Result<u64, CaptureError> {
        let io = |e: std::io::Error| CaptureError::Failed(format!("cannot write the video: {e}"));
        self.out
            .write_all(&(data.len() as u32).to_le_bytes())
            .map_err(io)?;
        self.out.write_all(&pts.to_le_bytes()).map_err(io)?;
        self.out.write_all(data).map_err(io)?;
        self.frames += 1;
        self.bytes += 12 + data.len() as u64;
        Ok(self.bytes)
    }

    /// Frames written so far.
    #[must_use]
    pub const fn frames(&self) -> u32 {
        self.frames
    }

    /// Patches the frame count and flushes; returns the file size.
    ///
    /// # Errors
    /// `Failed` on I/O errors.
    pub fn finish(mut self) -> Result<u64, CaptureError> {
        let io = |e: std::io::Error| CaptureError::Failed(format!("cannot finish the video: {e}"));
        self.out.flush().map_err(io)?;
        let file = self.out.get_mut();
        file.seek(SeekFrom::Start(24)).map_err(io)?;
        file.write_all(&self.frames.to_le_bytes()).map_err(io)?;
        file.flush().map_err(io)?;
        Ok(self.bytes)
    }
}

/// The header fields of an IVF file (for tests and diagnostics).
#[derive(Debug, PartialEq, Eq)]
pub struct IvfInfo {
    /// FourCC of the codec (`AV01`).
    pub fourcc: [u8; 4],
    /// Pixel width.
    pub width: u16,
    /// Pixel height.
    pub height: u16,
    /// Frame count in the header.
    pub header_frames: u32,
    /// Frames actually present (counted by walking the file).
    pub counted_frames: u32,
    /// Presentation timestamp of the last frame (time-base units).
    pub last_pts: u64,
}

/// Parses an IVF file, counting its frames.
///
/// # Errors
/// A message when the file is not a well-formed IVF.
pub fn inspect(bytes: &[u8]) -> Result<IvfInfo, String> {
    if bytes.len() < 32 || &bytes[..4] != b"DKIF" {
        return Err("not an IVF file".into());
    }
    let u16_at = |o: usize| u16::from_le_bytes([bytes[o], bytes[o + 1]]);
    let u32_at =
        |o: usize| u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
    let mut pos = 32usize;
    let mut counted = 0u32;
    let mut last_pts = 0u64;
    while pos + 12 <= bytes.len() {
        let size = u32_at(pos) as usize;
        last_pts = u64::from(u32_at(pos + 4)) | (u64::from(u32_at(pos + 8)) << 32);
        pos += 12 + size;
        if pos > bytes.len() {
            return Err("truncated frame".into());
        }
        counted += 1;
    }
    Ok(IvfInfo {
        fourcc: [bytes[8], bytes[9], bytes[10], bytes[11]],
        width: u16_at(12),
        height: u16_at(14),
        header_frames: u32_at(24),
        counted_frames: counted,
        last_pts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_frames_round_trip() {
        let path = std::env::temp_dir().join(format!("opts_ivf_{}.ivf", std::process::id()));
        let mut w = IvfWriter::create(&path, b"AV01", 320, 180, 30).expect("create");
        w.write_frame(0, &[1, 2, 3]).expect("frame");
        w.write_frame(1, &[4; 10]).expect("frame");
        let size = w.finish().expect("finish");
        let bytes = std::fs::read(&path).expect("read");
        assert_eq!(bytes.len() as u64, size);
        let info = inspect(&bytes).expect("inspect");
        assert_eq!(
            info,
            IvfInfo {
                fourcc: *b"AV01",
                width: 320,
                height: 180,
                header_frames: 2,
                counted_frames: 2,
                last_pts: 1
            }
        );
        assert!(
            inspect(&bytes[..bytes.len() - 1]).is_err(),
            "truncation is detected"
        );
    }
}
