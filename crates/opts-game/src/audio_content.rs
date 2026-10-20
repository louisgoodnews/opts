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

/// Where a clip stands.
#[derive(Debug, Clone, PartialEq)]
pub enum ClipState {
    /// Decoded; the handle plays.
    Ready(Handle<PcmClip>),
    /// Requested, a worker is decoding it (or it waits for one).
    Loading,
    /// Could not be read or decoded (the reason is in the log).
    Failed,
}

/// Decodes sound files on worker threads so a frame never waits for a file. A file is asked for
/// with [`ClipLoader::state`]; the first ask starts the decode, later asks answer
/// [`ClipState::Loading`] until [`poll_clip_loader`] has moved the finished clip into the
/// assets. At most [`ClipLoader::MAX_WORKERS`] decodes run at once.
#[derive(Resource)]
pub struct ClipLoader {
    states: HashMap<PathBuf, ClipState>,
    queue: std::collections::VecDeque<PathBuf>,
    running: usize,
    tx: std::sync::mpsc::Sender<(PathBuf, Result<PcmClip, String>)>,
    rx: std::sync::Mutex<std::sync::mpsc::Receiver<(PathBuf, Result<PcmClip, String>)>>,
    /// Bytes of decoded samples held (for the report; a limit comes with the eviction).
    pub bytes: usize,
}

impl Default for ClipLoader {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            states: HashMap::new(),
            queue: std::collections::VecDeque::new(),
            running: 0,
            tx,
            rx: std::sync::Mutex::new(rx),
            bytes: 0,
        }
    }
}

impl ClipLoader {
    /// Decodes running at the same time.
    pub const MAX_WORKERS: usize = 2;

    /// The state of `path`; asking for an unknown file starts its decode.
    pub fn state(&mut self, path: &Path) -> ClipState {
        if let Some(s) = self.states.get(path) {
            return s.clone();
        }
        self.states.insert(path.to_path_buf(), ClipState::Loading);
        self.queue.push_back(path.to_path_buf());
        self.start_queued();
        ClipState::Loading
    }

    fn start_queued(&mut self) {
        while self.running < Self::MAX_WORKERS {
            let Some(path) = self.queue.pop_front() else {
                return;
            };
            self.running += 1;
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let r = decode_file(&path);
                let _ = tx.send((path, r));
            });
        }
    }

    /// Counts by state: (ready, loading, failed).
    #[must_use]
    pub fn counts(&self) -> (usize, usize, usize) {
        self.states
            .values()
            .fold((0, 0, 0), |(r, l, f), s| match s {
                ClipState::Ready(_) => (r + 1, l, f),
                ClipState::Loading => (r, l + 1, f),
                ClipState::Failed => (r, l, f + 1),
            })
    }

    /// Moves finished decodes into `assets` and starts queued ones.
    pub fn poll(&mut self, assets: &mut Assets<PcmClip>) {
        let finished: Vec<(PathBuf, Result<PcmClip, String>)> = self
            .rx
            .lock()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        for (path, result) in finished {
            self.running = self.running.saturating_sub(1);
            let state = match result {
                Ok(clip) => {
                    self.bytes += clip.samples.len() * std::mem::size_of::<f32>();
                    ClipState::Ready(assets.add(clip))
                }
                Err(e) => {
                    tracing::warn!("sound not loaded: {e}");
                    ClipState::Failed
                }
            };
            self.states.insert(path, state);
        }
        self.start_queued();
    }
}

/// `Update` system: finishes the decodes the [`ClipLoader`] has done.
pub fn poll_clip_loader(mut loader: ResMut<ClipLoader>, mut assets: ResMut<Assets<PcmClip>>) {
    loader.poll(&mut assets);
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

    #[test]
    fn the_loader_decodes_on_a_worker_and_reports_states() {
        let dir = std::env::temp_dir().join(format!("opts-clip-loader-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let good = dir.join("beep.wav");
        std::fs::write(&good, wav_bytes()).expect("write");
        let bad = dir.join("none.wav");
        let mut assets = Assets::<PcmClip>::default();
        let mut loader = ClipLoader::default();
        assert_eq!(loader.state(&good), ClipState::Loading);
        assert_eq!(loader.state(&bad), ClipState::Loading);
        for _ in 0..200 {
            loader.poll(&mut assets);
            if loader.counts().1 == 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(loader.counts(), (1, 0, 1));
        let ClipState::Ready(h) = loader.state(&good) else {
            panic!("the good file is ready");
        };
        assert_eq!(assets.get(&h).map(PcmClip::frames), Some(4));
        assert_eq!(loader.state(&bad), ClipState::Failed);
        assert_eq!(loader.bytes, 16);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
