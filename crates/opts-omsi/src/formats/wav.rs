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
mod tests {
    use super::*;

    fn file(tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        b.extend_from_slice(b"WAVE");
        b.extend_from_slice(b"fmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&tag.to_le_bytes());
        b.extend_from_slice(&channels.to_le_bytes());
        b.extend_from_slice(&rate.to_le_bytes());
        b.extend_from_slice(&(rate * u32::from(channels) * u32::from(bits / 8)).to_le_bytes());
        b.extend_from_slice(&(channels * (bits / 8)).to_le_bytes());
        b.extend_from_slice(&bits.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data.len() as u32).to_le_bytes());
        b.extend_from_slice(data);
        b
    }

    #[test]
    fn pcm16_decodes_to_unit_range() {
        let data: Vec<u8> = [0i16, 16384, -32768, 32767]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        let w = decode(&file(1, 2, 44100, 16, &data)).expect("decodes");
        assert_eq!((w.channels, w.sample_rate), (2, 44100));
        assert_eq!(w.samples.len(), 4);
        assert_eq!(w.samples[0], 0.0);
        assert!((w.samples[1] - 0.5).abs() < 1e-6);
        assert_eq!(w.samples[2], -1.0);
        assert!(w.samples[3] < 1.0 && w.samples[3] > 0.999);
        assert!((w.seconds() - 2.0 / 44100.0).abs() < 1e-9);
    }

    #[test]
    fn eight_bit_is_unsigned_around_128() {
        let w = decode(&file(1, 1, 8000, 8, &[128, 255, 0])).expect("decodes");
        assert_eq!(w.samples[0], 0.0);
        assert!(w.samples[1] > 0.99);
        assert_eq!(w.samples[2], -1.0);
    }

    #[test]
    fn pcm24_and_float32_decode() {
        // 24 bit: 0x400000 is half scale.
        let w = decode(&file(
            1,
            1,
            48000,
            24,
            &[0x00, 0x00, 0x40, 0x00, 0x00, 0xC0],
        ))
        .expect("24");
        assert!((w.samples[0] - 0.5).abs() < 1e-6);
        assert!((w.samples[1] + 0.5).abs() < 1e-6);
        let data: Vec<u8> = [0.25f32, -2.0]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        let w = decode(&file(3, 1, 96000, 32, &data)).expect("float");
        assert_eq!(w.samples, vec![0.25, -1.0]);
    }

    #[test]
    fn a_partial_last_frame_and_a_short_data_chunk_are_tolerated() {
        // Stereo with 3 samples: the odd one is dropped.
        let data: Vec<u8> = [1i16, 2, 3].iter().flat_map(|s| s.to_le_bytes()).collect();
        let w = decode(&file(1, 2, 22050, 16, &data)).expect("decodes");
        assert_eq!(w.samples.len(), 2);
        // The data chunk claims more than the file holds.
        let mut b = file(1, 1, 22050, 16, &[1, 0, 2, 0]);
        let at = b.len() - 4 - 4;
        b[at..at + 4].copy_from_slice(&1000u32.to_le_bytes());
        assert_eq!(decode(&b).expect("decodes").samples.len(), 2);
    }

    #[test]
    fn what_is_not_a_wav_is_named() {
        assert_eq!(
            decode(&[0xFF, 0xF2, 0x40, 0xC0, 0, 0, 0, 0, 0, 0, 0, 0]),
            Err(WavError::NotRiff)
        );
        assert_eq!(
            decode(&file(2, 1, 8000, 4, &[0, 0])),
            Err(WavError::Unsupported { tag: 2, bits: 4 })
        );
        let mut no_data = file(1, 1, 8000, 16, &[]);
        no_data.truncate(36);
        assert_eq!(decode(&no_data), Err(WavError::NoData));
    }

    #[test]
    fn odd_sized_chunks_are_padded() {
        let mut b = file(1, 1, 8000, 16, &[]);
        b.truncate(36);
        b.extend_from_slice(b"LIST");
        b.extend_from_slice(&3u32.to_le_bytes());
        b.extend_from_slice(&[1, 2, 3, 0]); // three bytes and the pad byte
        b.extend_from_slice(b"data");
        b.extend_from_slice(&2u32.to_le_bytes());
        b.extend_from_slice(&[0x00, 0x40]);
        let w = decode(&b).expect("decodes");
        assert!((w.samples[0] - 0.5).abs() < 1e-6);
    }
}
