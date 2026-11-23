//! `.wav` sound files (`docs/src/formats/wav.md`): a RIFF/WAVE walk and a decode to interleaved
//! `f32` samples.
//!
//! OMSI adds nothing to the container. The corpus holds PCM 8, 16 and 24 bit and IEEE float 32 bit,
//! mono or stereo, at rates from 4 kHz to 96 kHz; five files named `.wav` are MPEG audio without a
//! RIFF header and are reported as [`WavError::NotRiff`], not decoded.

use std::fmt;

/// A decoded sound: interleaved samples in -1 to 1.
#[derive(Debug, Clone, PartialEq)]
pub struct Wav {
    /// 1 or 2 (the corpus has no more).
    pub channels: u16,
    /// Frames per second.
    pub sample_rate: u32,
    /// Interleaved samples, `frames × channels` of them.
    pub samples: Vec<f32>,
}

impl Wav {
    /// Length in seconds.
    #[must_use]
    pub fn seconds(&self) -> f32 {
        if self.channels == 0 || self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.len() as f32 / (f32::from(self.channels) * self.sample_rate as f32)
    }
}

/// Why a file could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WavError {
    /// No `RIFF`/`WAVE` header (an MPEG file under a `.wav` name, or something else).
    NotRiff,
    /// The `fmt ` chunk is missing, too short or says something impossible.
    BadFormat(String),
    /// The `data` chunk is missing.
    NoData,
    /// A sample format outside the corpus (tag and bits per sample).
    Unsupported {
        /// The `fmt ` format tag.
        tag: u16,
        /// Bits per sample.
        bits: u16,
    },
}

impl fmt::Display for WavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRiff => f.write_str("not a RIFF/WAVE file"),
            Self::BadFormat(why) => write!(f, "bad fmt chunk: {why}"),
            Self::NoData => f.write_str("no data chunk"),
            Self::Unsupported { tag, bits } => {
                write!(f, "unsupported sample format (tag {tag}, {bits} bits)")
            }
        }
    }
}

impl std::error::Error for WavError {}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Decodes a RIFF/WAVE file. A `data` chunk that claims more than the file holds is read to the
/// end of the file (truncated downloads and editors that never patched the size exist).
///
/// # Errors
/// [`WavError`] for anything that is not a PCM 8/16/24 bit or float 32 bit RIFF/WAVE file.
pub fn decode(bytes: &[u8]) -> Result<Wav, WavError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(WavError::NotRiff);
    }
    let mut at = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32_at(bytes, at + 4).unwrap_or(0) as usize;
        let body_start = at + 8;
        let body_end = body_start.saturating_add(size).min(bytes.len());
        let body = &bytes[body_start..body_end];
        if id == b"fmt " {
            let tag = u16_at(body, 0).ok_or_else(|| WavError::BadFormat("too short".into()))?;
            let channels =
                u16_at(body, 2).ok_or_else(|| WavError::BadFormat("too short".into()))?;
            let rate = u32_at(body, 4).ok_or_else(|| WavError::BadFormat("too short".into()))?;
            let bits = u16_at(body, 14).ok_or_else(|| WavError::BadFormat("too short".into()))?;
            fmt = Some((tag, channels, rate, bits));
        } else if id == b"data" {
            data = Some(body);
            break;
        }
        // Chunks are padded to an even size.
        at = body_start.saturating_add(size).saturating_add(size & 1);
    }
    let (tag, channels, sample_rate, bits) =
        fmt.ok_or_else(|| WavError::BadFormat("missing".into()))?;
    if channels == 0 || sample_rate == 0 {
        return Err(WavError::BadFormat("zero channels or rate".into()));
    }
    let data = data.ok_or(WavError::NoData)?;
    let samples: Vec<f32> = match (tag, bits) {
        (1, 8) => data
            .iter()
            .map(|b| (f32::from(*b) - 128.0) / 128.0)
            .collect(),
        (1, 16) => data
            .chunks_exact(2)
            .map(|c| f32::from(i16::from_le_bytes([c[0], c[1]])) / 32768.0)
            .collect(),
        (1, 24) => data
            .chunks_exact(3)
            .map(|c| {
                let v = i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8;
                v as f32 / 8_388_608.0
            })
            .collect(),
        (3, 32) => data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]).clamp(-1.0, 1.0))
            .collect(),
        _ => return Err(WavError::Unsupported { tag, bits }),
    };
    // A last partial frame is dropped: spans must end on frame boundaries.
    let whole = samples.len() - samples.len() % usize::from(channels);
    let mut samples = samples;
    samples.truncate(whole);
    Ok(Wav {
        channels,
        sample_rate,
        samples,
    })
}

#[cfg(test)]
mod tests;
