//! AV1 recording: RGB frames to YUV 4:2:0, encoded with `rav1e` (pure Rust build, no assembly,
//! so no `nasm` is needed), muxed into IVF.

use std::path::Path;

use rav1e::prelude::{
    ChromaSampling, ColorDescription, ColorPrimaries, Config, Context, EncoderConfig,
    EncoderStatus, MatrixCoefficients, PixelRange, Rational, TransferCharacteristics,
};

use super::encoder::{Encoder, FrameInfo, RgbFrame};
use super::error::CaptureError;
use super::ivf::IvfWriter;

/// `rav1e` speed preset: 10 is the fastest; recording shares the CPU with the simulation.
const SPEED: u8 = 10;
/// Threads `rav1e` may use.
const THREADS: usize = 4;

/// Maps the 1..=100 `quality` parameter to a `rav1e` quantizer (lower is better).
#[must_use]
pub fn quantizer_for(quality: u8) -> usize {
    let q = i32::from(quality.clamp(1, 100));
    (230 - q * 190 / 100).clamp(40, 230) as usize
}

fn fail(what: &str, e: impl std::fmt::Display) -> CaptureError {
    CaptureError::Failed(format!("AV1 {what} failed: {e}"))
}

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

/// AV1 in IVF.
pub struct Av1Ivf {
    ctx: Context<u8>,
    out: IvfWriter,
    width: usize,
    height: usize,
    fps: u64,
    /// Presentation time (in 1/fps units) of every frame sent, by input frame number.
    pts: Vec<u64>,
}

impl Av1Ivf {
    /// Opens `path` for a stream of `width` x `height` at `fps`.
    ///
    /// # Errors
    /// `Failed` when the encoder rejects the configuration or the file cannot be created.
    pub fn create(
        path: &Path,
        width: u32,
        height: u32,
        fps: u32,
        quality: u8,
    ) -> Result<Self, CaptureError> {
        let mut enc = EncoderConfig::with_speed_preset(SPEED);
        enc.width = width as usize;
        enc.height = height as usize;
        enc.bit_depth = 8;
        enc.chroma_sampling = ChromaSampling::Cs420;
        enc.pixel_range = PixelRange::Limited;
        enc.color_description = Some(ColorDescription {
            color_primaries: ColorPrimaries::BT709,
            transfer_characteristics: TransferCharacteristics::BT709,
            matrix_coefficients: MatrixCoefficients::BT709,
        });
        enc.time_base = Rational::new(1, u64::from(fps.max(1)));
        enc.quantizer = quantizer_for(quality);
        enc.low_latency = true;
        enc.max_key_frame_interval = u64::from(fps.max(1)) * 2;
        // Tiles let several threads work on one frame; without assembly a 1080p frame is slow
        // on one thread. Small frames stay in one tile.
        if width >= 960 {
            enc.tile_cols = 4;
            enc.tile_rows = 1;
        }
        let cfg = Config::new().with_encoder_config(enc).with_threads(THREADS);
        let ctx: Context<u8> = cfg.new_context().map_err(|e| fail("setup", e))?;
        let w16 = u16::try_from(width).map_err(|_| fail("setup", "frame wider than 65535"))?;
        let h16 = u16::try_from(height).map_err(|_| fail("setup", "frame taller than 65535"))?;
        Ok(Self {
            ctx,
            out: IvfWriter::create(path, b"AV01", w16, h16, fps.max(1))?,
            width: width as usize,
            height: height as usize,
            fps: u64::from(fps.max(1)),
            pts: Vec::new(),
        })
    }

    fn drain(&mut self, flushing: bool) -> Result<u64, CaptureError> {
        let mut bytes = 0;
        loop {
            match self.ctx.receive_packet() {
                Ok(packet) => {
                    let pts = self
                        .pts
                        .get(packet.input_frameno as usize)
                        .copied()
                        .unwrap_or(packet.input_frameno);
                    bytes = self.out.write_frame(pts, &packet.data)?;
                }
                Err(EncoderStatus::Encoded) => {}
                Err(EncoderStatus::NeedMoreData) if !flushing => return Ok(bytes),
                Err(EncoderStatus::NeedMoreData | EncoderStatus::LimitReached) => return Ok(bytes),
                Err(e) => return Err(fail("encoding", e)),
            }
        }
    }
}

impl Encoder for Av1Ivf {
    fn push(&mut self, frame: &RgbFrame, info: &FrameInfo) -> Result<u64, CaptureError> {
        let frame = if (frame.width as usize, frame.height as usize) == (self.width, self.height) {
            std::borrow::Cow::Borrowed(frame)
        } else {
            std::borrow::Cow::Owned(frame.clone().fitted(self.width as u32, self.height as u32))
        };
        let [y, u, v] = to_yuv420(&frame);
        let mut f = self.ctx.new_frame();
        f.planes[0].copy_from_raw_u8(&y, self.width, 1);
        f.planes[1].copy_from_raw_u8(&u, self.width.div_ceil(2), 1);
        f.planes[2].copy_from_raw_u8(&v, self.width.div_ceil(2), 1);
        // Presentation time follows the capture clock, so dropped frames leave a gap instead of
        // squeezing the video (variable frame rate); it never goes backwards.
        let wanted = (info.wall_ms * self.fps + 500) / 1000;
        let pts = self.pts.last().map_or(wanted, |last| wanted.max(last + 1));
        self.pts.push(pts);
        self.ctx.send_frame(f).map_err(|e| fail("encoding", e))?;
        let written = self.drain(false)?;
        Ok(written.max(self.out.frames().into()))
    }

    fn finish(mut self: Box<Self>) -> Result<u64, CaptureError> {
        self.ctx.flush();
        self.drain(true)?;
        self.out.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::ivf;

    fn gradient(w: u32, h: u32, t: u32) -> RgbFrame {
        let mut data = Vec::new();
        for y in 0..h {
            for x in 0..w {
                data.extend_from_slice(&[((x + t * 4) % 256) as u8, (y * 255 / h) as u8, 90]);
            }
        }
        RgbFrame {
            width: w,
            height: h,
            data,
        }
    }

    #[test]
    fn dropped_frames_leave_a_gap_in_presentation_time() {
        let path = std::env::temp_dir().join(format!("opts_av1_vfr_{}.ivf", std::process::id()));
        let mut enc = Av1Ivf::create(&path, 64, 48, 30, 60).expect("create");
        // 30 fps clock; frames 1 and 2 were dropped, so the third arrives at 100 ms.
        for (i, wall_ms) in [0u64, 33, 133, 166].into_iter().enumerate() {
            let info = FrameInfo {
                index: i as u64,
                sim_time: 0.0,
                frame: 0,
                wall_ms,
            };
            enc.push(&gradient(64, 48, i as u32), &info).expect("push");
        }
        Box::new(enc).finish().expect("finish");
        let i = ivf::inspect(&std::fs::read(&path).expect("read")).expect("ivf");
        assert_eq!(i.counted_frames, 4);
        assert_eq!(
            i.last_pts, 5,
            "166 ms at 30 fps is tick 5, not frame number 3"
        );
    }

    #[test]
    fn quantizer_follows_quality() {
        assert!(quantizer_for(100) < quantizer_for(50));
        assert!(quantizer_for(50) < quantizer_for(1));
    }

    #[test]
    fn white_and_black_map_to_the_limited_range() {
        let white = RgbFrame {
            width: 2,
            height: 2,
            data: vec![255; 12],
        };
        let black = RgbFrame {
            width: 2,
            height: 2,
            data: vec![0; 12],
        };
        assert_eq!(to_yuv420(&white)[0][0], 235);
        assert_eq!(to_yuv420(&black)[0][0], 16);
        assert_eq!(to_yuv420(&white)[1][0], 128);
    }

    #[test]
    fn a_short_clip_is_a_well_formed_ivf_with_every_frame() {
        let path = std::env::temp_dir().join(format!("opts_av1_{}.ivf", std::process::id()));
        let mut enc = Av1Ivf::create(&path, 160, 90, 30, 60).expect("create");
        let info = FrameInfo {
            index: 0,
            sim_time: 0.0,
            frame: 0,
            wall_ms: 0,
        };
        for t in 0..6 {
            enc.push(&gradient(160, 90, t), &info).expect("push");
        }
        Box::new(enc).finish().expect("finish");
        let bytes = std::fs::read(&path).expect("read");
        let i = ivf::inspect(&bytes).expect("ivf");
        assert_eq!((i.fourcc, i.width, i.height), (*b"AV01", 160, 90));
        assert_eq!(i.header_frames, 6);
        assert_eq!(i.counted_frames, 6);
    }
}
