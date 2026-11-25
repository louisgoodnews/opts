//! [`PcmClip`]: decoded audio held as an asset and played through Bevy's audio output.
//!
//! OMSI sounds come as RIFF/WAVE files in several sample formats; the adapter in `opts-game`
//! decodes them to interleaved `f32` and hands the engine this neutral clip, so the engine never
//! depends on a file format and Bevy needs none of its optional decoders.

use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

use bevy::audio::{ChannelCount, Decodable, Sample, SampleRate, Source};
use bevy::prelude::*;

/// Interleaved samples in -1 to 1, shared between all players of the clip.
#[derive(Asset, TypePath, Debug, Clone)]
pub struct PcmClip {
    /// `frames × channels` samples.
    pub samples: Arc<[f32]>,
    /// 1 or 2.
    pub channels: u16,
    /// Frames per second.
    pub sample_rate: u32,
}

impl PcmClip {
    /// A clip; `channels` and `sample_rate` of 0 are raised to 1, and a partial last frame is cut.
    #[must_use]
    pub fn new(mut samples: Vec<f32>, channels: u16, sample_rate: u32) -> Self {
        let channels = channels.max(1);
        let whole = samples.len() - samples.len() % usize::from(channels);
        samples.truncate(whole);
        Self {
            samples: samples.into(),
            channels,
            sample_rate: sample_rate.max(1),
        }
    }

    /// A mono sine tone (test signal and the engine's own beep).
    #[must_use]
    pub fn tone(frequency_hz: f32, seconds: f32, sample_rate: u32) -> Self {
        let n = (seconds.max(0.0) * sample_rate as f32) as usize;
        let samples = (0..n)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                // A short fade at both ends keeps the tone from clicking.
                let fade = (i.min(n - 1 - i) as f32 / (0.01 * sample_rate as f32)).min(1.0);
                (std::f32::consts::TAU * frequency_hz * t).sin() * 0.5 * fade
            })
            .collect();
        Self::new(samples, 1, sample_rate)
    }

    /// Length in seconds.
    #[must_use]
    pub fn seconds(&self) -> f32 {
        self.samples.len() as f32 / (f32::from(self.channels) * self.sample_rate as f32)
    }

    /// Number of frames.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }
}

/// The player's view of a clip.
pub struct ClipDecoder {
    samples: Arc<[f32]>,
    at: usize,
    channels: ChannelCount,
    rate: SampleRate,
}

impl Iterator for ClipDecoder {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let s = self.samples.get(self.at).copied()?;
        self.at += 1;
        Some(s)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.samples.len() - self.at;
        (left, Some(left))
    }
}

impl Source for ClipDecoder {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.samples.len() - self.at)
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let frames = self.samples.len() / usize::from(self.channels.get());
        Some(Duration::from_secs_f64(
            frames as f64 / f64::from(self.rate.get()),
        ))
    }
}

impl Decodable for PcmClip {
    type Decoder = ClipDecoder;

    fn decoder(&self) -> ClipDecoder {
        ClipDecoder {
            samples: self.samples.clone(),
            at: 0,
            channels: NonZero::new(self.channels).unwrap_or(NonZero::<u16>::MIN),
            rate: NonZero::new(self.sample_rate).unwrap_or(NonZero::<u32>::MIN),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clip_cuts_a_partial_frame_and_reports_its_length() {
        let c = PcmClip::new(vec![0.0; 7], 2, 100);
        assert_eq!(c.frames(), 3);
        assert!((c.seconds() - 0.03).abs() < 1e-6);
    }

    #[test]
    fn the_decoder_yields_every_sample_once_with_the_clips_format() {
        let c = PcmClip::new(vec![0.1, 0.2, 0.3, 0.4], 2, 44100);
        let d = c.decoder();
        assert_eq!(d.channels().get(), 2);
        assert_eq!(d.sample_rate().get(), 44100);
        assert_eq!(d.current_span_len(), Some(4));
        assert_eq!(d.collect::<Vec<_>>(), vec![0.1, 0.2, 0.3, 0.4]);
    }

    #[test]
    fn a_tone_has_the_asked_length_and_fades_at_the_ends() {
        let c = PcmClip::tone(440.0, 0.5, 8000);
        assert_eq!(c.frames(), 4000);
        assert_eq!(c.samples[0], 0.0);
        assert!(c.samples.iter().all(|s| s.abs() <= 0.5));
        assert!(c.samples[2000..2100].iter().any(|s| s.abs() > 0.3));
    }
}
