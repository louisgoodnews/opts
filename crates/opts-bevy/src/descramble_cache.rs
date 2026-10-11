//! On-disk cache of descrambled meshes.
//!
//! Decoding a keyed `.o3d` takes tens to thousands of milliseconds, so the outcome (accepted
//! geometry or a rejection) is stored per file content under the user's cache directory and
//! reused by later sessions. The cache only holds data derived from the user's own installation
//! and never leaves their machine.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use opts_core::formats::o3d::descramble::{decode_confident_with, DecodeParams};
use opts_core::formats::o3d::O3d;

/// Bump when the decoder *algorithm* changes in a way that alters its output; parameter changes
/// are covered by [`DecodeParams::fingerprint`], which is part of every entry's file name.
const VERSION: u8 = 3;
/// Default size cap of the cache directory.
pub const DEFAULT_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// Identifies one model of one file: content hash and model index.
pub type Key = (u64, usize);

/// Cache key of model `model_index` in a file with these bytes.
#[must_use]
pub fn key(file_bytes: &[u8], model_index: usize) -> Key {
    (fnv(file_bytes), model_index)
}
const MAGIC: &[u8; 4] = b"OPDC";

/// A cached decoding outcome for one mesh.
#[derive(Debug, Clone, PartialEq)]
pub enum Cached {
    /// The decoder rejected the mesh.
    Rejected,
    /// Decoded positions, UVs and the triangles that were kept (by original index).
    Decoded {
        positions: Vec<[f32; 3]>,
        uvs: Vec<[f32; 2]>,
        kept: Vec<bool>,
    },
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

fn dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("opts").join("descramble"))
}

fn file_for(key: Key, fingerprint: u32) -> Option<PathBuf> {
    Some(dir()?.join(format!("{:016x}-{}-{fingerprint:08x}.bin", key.0, key.1)))
}

/// Deletes the least recently used entries until the directory fits `max_bytes`; returns how
/// many files were removed.
pub fn prune(max_bytes: u64) -> usize {
    let Some(dir) = dir() else { return 0 };
    let mut files: Vec<(SystemTime, u64, PathBuf)> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            Some((meta.modified().ok()?, meta.len(), e.path()))
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    files.sort();
    let mut removed = 0;
    for (_, len, path) in files {
        if total <= max_bytes {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total -= len;
            removed += 1;
        }
    }
    removed
}

/// Reads the cached outcome of `key` decoded under the parameters with this `fingerprint`.
#[must_use]
pub fn load(key: Key, fingerprint: u32) -> Option<Cached> {
    let path = file_for(key, fingerprint)?;
    let data = std::fs::read(&path).ok()?;
    // Refresh the modification time: pruning removes the least recently used entries first.
    if let Ok(f) = std::fs::OpenOptions::new().write(true).open(&path) {
        let _ = f.set_modified(SystemTime::now());
    }
    if data.len() < 6 || &data[..4] != MAGIC || data[4] != VERSION {
        return None;
    }
    if data[5] == 0 {
        return Some(Cached::Rejected);
    }
    let n = u32::from_le_bytes(data.get(6..10)?.try_into().ok()?) as usize;
    let mut at = 10;
    let mut f32s = |count: usize| -> Option<Vec<f32>> {
        let end = at + count * 4;
        let out = data
            .get(at..end)?
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        at = end;
        Some(out)
    };
    let pos = f32s(n * 3)?;
    let uv = f32s(n * 2)?;
    let faces = u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?) as usize;
    let kept = data
        .get(at + 4..at + 4 + faces)?
        .iter()
        .map(|b| *b != 0)
        .collect();
    Some(Cached::Decoded {
        positions: pos.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect(),
        uvs: uv.chunks_exact(2).map(|c| [c[0], c[1]]).collect(),
        kept,
    })
}

/// Stores an outcome; failures to write are ignored (the cache is an optimisation).
pub fn store(key: Key, fingerprint: u32, value: &Cached) {
    let Some(path) = file_for(key, fingerprint) else {
        return;
    };
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    match value {
        Cached::Rejected => out.push(0),
        Cached::Decoded {
            positions,
            uvs,
            kept,
        } => {
            out.push(1);
            out.extend_from_slice(&(positions.len() as u32).to_le_bytes());
            for p in positions {
                for c in p {
                    out.extend_from_slice(&c.to_le_bytes());
                }
            }
            for t in uvs {
                for c in t {
                    out.extend_from_slice(&c.to_le_bytes());
                }
            }
            out.extend_from_slice(&(kept.len() as u32).to_le_bytes());
            out.extend(kept.iter().map(|k| u8::from(*k)));
        }
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, out);
}

/// Captures the result of decoding `decoded` from the original `original` mesh.
#[must_use]
pub fn capture(original: &O3d, decoded: &O3d) -> Cached {
    // `decode_confident` keeps vertex order and only removes triangles: record which survive.
    let mut remaining = decoded.indices.iter().peekable();
    let mut kept = Vec::with_capacity(original.indices.len());
    for f in &original.indices {
        if remaining.peek().is_some_and(|r| *r == f) {
            remaining.next();
            kept.push(true);
        } else {
            kept.push(false);
        }
    }
    Cached::Decoded {
        positions: decoded
            .vertices
            .iter()
            .map(|v| v.position.to_array())
            .collect(),
        uvs: decoded.vertices.iter().map(|v| v.uv.to_array()).collect(),
        kept,
    }
}

/// Applies a cached decoding to `o`; returns `false` if it does not fit the mesh.
pub fn apply(o: &mut O3d, cached: &Cached) -> bool {
    let Cached::Decoded {
        positions,
        uvs,
        kept,
    } = cached
    else {
        return false;
    };
    if positions.len() != o.vertices.len()
        || uvs.len() != o.vertices.len()
        || kept.len() != o.indices.len()
    {
        return false;
    }
    for ((v, p), t) in o.vertices.iter_mut().zip(positions).zip(uvs) {
        v.position = (*p).into();
        v.uv = (*t).into();
    }
    let (mut idx, mut mats) = (Vec::new(), Vec::new());
    for ((f, m), k) in o.indices.iter().zip(&o.face_materials).zip(kept) {
        if *k {
            idx.push(*f);
            mats.push(*m);
        }
    }
    o.indices = idx;
    o.face_materials = mats;
    o.descrambled = true;
    true
}

/// Background decoder: keyed meshes without a plain twin are decoded on worker threads so tile
/// loading never waits for them. Results go to the on-disk cache and stay in memory, so the
/// loader finds them on its next attempt.
pub struct DecodePool {
    jobs: Sender<(Key, Box<O3d>)>,
    state: Arc<Mutex<PoolState>>,
    params: DecodeParams,
}

#[derive(Default)]
struct PoolState {
    pending: HashSet<Key>,
    results: HashMap<Key, Cached>,
    finished: Vec<Key>,
}

impl DecodePool {
    /// Starts `threads` workers decoding with `params`.
    #[must_use]
    pub fn new(threads: usize, params: DecodeParams) -> Self {
        let (tx, rx) = channel::<(Key, Box<O3d>)>();
        let rx = Arc::new(Mutex::new(rx));
        let state = Arc::new(Mutex::new(PoolState::default()));
        let fingerprint = params.fingerprint();
        for i in 0..threads.max(1) {
            let (rx, state, params) = (rx.clone(), state.clone(), params.clone());
            std::thread::Builder::new()
                .name(format!("opts-descramble-{i}"))
                .spawn(move || loop {
                    let job = rx.lock().ok().and_then(|r| r.recv().ok());
                    let Some((key, mut model)) = job else { return };
                    let before = (*model).clone();
                    let t = std::time::Instant::now();
                    let report = decode_confident_with(&mut model, &params);
                    let outcome = if report.accepted {
                        capture(&before, &model)
                    } else {
                        Cached::Rejected
                    };
                    tracing::info!(
                        accepted = report.accepted,
                        agreement = report.agreement,
                        ms = t.elapsed().as_millis() as u64,
                        "scrambled mesh decoded (background)"
                    );
                    store(key, fingerprint, &outcome);
                    if let Ok(mut st) = state.lock() {
                        st.pending.remove(&key);
                        st.results.insert(key, outcome);
                        st.finished.push(key);
                    }
                })
                .ok();
        }
        // Keep the cache directory bounded; done off the loading path.
        std::thread::spawn(|| {
            prune(DEFAULT_MAX_BYTES);
        });
        Self {
            jobs: tx,
            state,
            params,
        }
    }

    /// Parameters the workers decode with.
    #[must_use]
    pub fn params(&self) -> &DecodeParams {
        &self.params
    }

    /// Finished outcome of `key`, if any.
    #[must_use]
    pub fn result(&self, key: Key) -> Option<Cached> {
        self.state.lock().ok()?.results.get(&key).cloned()
    }

    /// Queues `model` unless `key` is already queued or finished.
    pub fn submit(&self, key: Key, model: &O3d) {
        let Ok(mut st) = self.state.lock() else {
            return;
        };
        if st.pending.contains(&key) || st.results.contains_key(&key) {
            return;
        }
        st.pending.insert(key);
        drop(st);
        let _ = self.jobs.send((key, Box::new(model.clone())));
    }

    /// Whether `key` is queued or running.
    #[must_use]
    pub fn is_pending(&self, key: Key) -> bool {
        self.state.lock().is_ok_and(|st| st.pending.contains(&key))
    }

    /// Number of meshes queued or running.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.state.lock().map_or(0, |st| st.pending.len())
    }

    /// Keys finished since the last call.
    pub fn drain_finished(&self) -> Vec<Key> {
        self.state
            .lock()
            .map(|mut st| std::mem::take(&mut st.finished))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both tests point `XDG_CACHE_HOME` somewhere; they must not interleave.
    static ENV: Mutex<()> = Mutex::new(());

    #[test]
    fn round_trips_a_decoded_outcome() {
        let _lock = ENV
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _g = std::env::temp_dir().join(format!("opts-cache-test-{}", std::process::id()));
        std::env::set_var("XDG_CACHE_HOME", &_g);
        let value = Cached::Decoded {
            positions: vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
            uvs: vec![[0.25, 0.5], [0.75, 1.0]],
            kept: vec![true, false, true],
        };
        store(key(b"file-a", 0), 7, &value);
        store(key(b"file-b", 1), 7, &Cached::Rejected);
        assert_eq!(load(key(b"file-a", 0), 7), Some(value));
        assert_eq!(load(key(b"file-b", 1), 7), Some(Cached::Rejected));
        assert_eq!(load(key(b"file-a", 1), 7), None);
        // Another parameter set never sees the entry.
        assert_eq!(load(key(b"file-a", 0), 8), None);
        // Pruning to zero removes everything.
        assert_eq!(prune(0), 2);
        assert_eq!(load(key(b"file-b", 1), 7), None);
        let _ = std::fs::remove_dir_all(_g);
    }

    #[test]
    fn pool_decodes_in_the_background_and_reports_completion() {
        let _lock = ENV
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = std::env::temp_dir().join(format!("opts-pool-test-{}", std::process::id()));
        std::env::set_var("XDG_CACHE_HOME", &dir);
        let pool = DecodePool::new(1, DecodeParams::default());
        // Too small for the decoder: it rejects the mesh, which is still a (cached) outcome.
        let mesh = O3d {
            version: 4,
            header_flags: 0,
            vertices: Vec::new(),
            indices: Vec::new(),
            face_materials: Vec::new(),
            materials: Vec::new(),
            transform: None,
            encryption_key: Some(1),
            descrambled: false,
            unknown_chunks: Vec::new(),
        };
        let k = key(b"pool-file", 0);
        pool.submit(k, &mesh);
        pool.submit(k, &mesh); // duplicate submissions are ignored
        let start = std::time::Instant::now();
        let mut finished = Vec::new();
        while finished.is_empty() && start.elapsed().as_secs() < 10 {
            finished = pool.drain_finished();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(finished, vec![k]);
        assert!(!pool.is_pending(k));
        assert_eq!(pool.pending_count(), 0);
        assert_eq!(pool.result(k), Some(Cached::Rejected));
        assert_eq!(load(k, pool.params().fingerprint()), Some(Cached::Rejected));
        let _ = std::fs::remove_dir_all(dir);
    }
}
