//! OMSI sounds as engine clips (M-0027): reads a `.wav`, decodes it with
//! `opts_core::formats::wav` and keeps one [`PcmClip`] handle per file, so every emitter that
//! names the same file shares the samples. The engine itself (`opts_bevy::audio`) knows no file
//! format; this is the adapter, like the model and texture loaders.
//!
//! Decoding is done where the call is made. A map has up to a few hundred megabytes of sounds,
//! so the milestones that place them (scenery, traffic, vehicles) decode on a loader thread and
//! only the `Handle` lookup happens on the frame.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use opts_bevy::PcmClip;

/// Decodes a file to a clip.
///
/// # Errors
/// The read error or the decoder's reason (an MPEG file under a `.wav` name, an unusual sample
/// format).
pub fn decode_file(path: &Path) -> Result<PcmClip, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let wav =
        opts_core::formats::wav::decode(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(PcmClip::new(wav.samples, wav.channels, wav.sample_rate))
}

/// One handle per file; failures are remembered so a broken file is reported once.
#[derive(Resource, Default)]
pub struct ClipCache {
    clips: HashMap<PathBuf, Handle<PcmClip>>,
    failed: HashSet<PathBuf>,
}

impl ClipCache {
    /// The clip of `path`, decoded on first use.
    ///
    /// # Errors
    /// The decoder's reason, the first time; later calls for the same file answer `Err` with
    /// "failed before".
    pub fn clip(
        &mut self,
        path: &Path,
        assets: &mut Assets<PcmClip>,
    ) -> Result<Handle<PcmClip>, String> {
        if let Some(h) = self.clips.get(path) {
            return Ok(h.clone());
        }
        if self.failed.contains(path) {
            return Err(format!("{}: failed before", path.display()));
        }
        match decode_file(path) {
            Ok(clip) => {
                let h = assets.add(clip);
                self.clips.insert(path.to_path_buf(), h.clone());
                Ok(h)
            }
            Err(e) => {
                self.failed.insert(path.to_path_buf());
                tracing::warn!("sound not loaded: {e}");
                Err(e)
            }
        }
    }

    /// How many files are decoded and held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.clips.len()
    }

    /// Whether no file is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_bytes() -> Vec<u8> {
        let data: Vec<u8> = [0i16, 8192, -8192, 0]
            .iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&22050u32.to_le_bytes());
        b.extend_from_slice(&44100u32.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data.len() as u32).to_le_bytes());
        b.extend_from_slice(&data);
        b
    }

    #[test]
    fn a_file_is_decoded_once_and_shared() {
        let dir = std::env::temp_dir().join(format!("opts-clip-cache-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let good = dir.join("beep.wav");
        let bad = dir.join("mpeg.wav");
        std::fs::write(&good, wav_bytes()).expect("write");
        std::fs::write(&bad, [0xFF, 0xF2, 0x40, 0xC0, 1, 2, 3, 4, 5, 6, 7, 8]).expect("write");
        let mut assets = Assets::<PcmClip>::default();
        let mut cache = ClipCache::default();
        let a = cache.clip(&good, &mut assets).expect("decodes");
        let b = cache.clip(&good, &mut assets).expect("cached");
        assert_eq!(a, b);
        assert_eq!(cache.len(), 1);
        let clip = assets.get(&a).expect("asset");
        assert_eq!(
            (clip.channels, clip.sample_rate, clip.frames()),
            (1, 22050, 4)
        );
        // A file that is not RIFF/WAVE is refused with a reason and remembered.
        let e = cache.clip(&bad, &mut assets).expect_err("refused");
        assert!(e.contains("not a RIFF/WAVE"), "{e}");
        let e2 = cache.clip(&bad, &mut assets).expect_err("refused again");
        assert!(e2.contains("failed before"), "{e2}");
        assert!(cache.clip(&dir.join("none.wav"), &mut assets).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
