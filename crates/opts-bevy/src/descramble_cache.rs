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

use opts_core::formats::o3d::descramble::{
    decode_confident_bounded, decode_confident_with, decode_joint_bounded, same_model,
    DecodeParams, ExpectedBounds,
};
use opts_core::formats::o3d::O3d;

/// Bump when the decoder *algorithm* changes in a way that alters its output; parameter changes
/// are covered by [`DecodeParams::fingerprint`], which is part of every entry's file name.
/// 10: decoded meshes are put into the model frame by the vertex-0 anchor (`anchor_frame`).
const VERSION: u8 = 10;
/// Default size cap of the cache directory.
pub const DEFAULT_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// Identifies one model of one file: content hash and model index.
pub type Key = (u64, usize);

/// Cache key of model `model_index` in a file with these bytes.
#[must_use]
pub fn key(file_bytes: &[u8], model_index: usize) -> Key {
    (fnv(file_bytes), model_index)
}

/// Like [`key`], for a decoding restricted to `bounds` (a different box is a different result).
#[must_use]
pub fn key_bounded(file_bytes: &[u8], model_index: usize, bounds: Option<ExpectedBounds>) -> Key {
    let (h, i) = key(file_bytes, model_index);
    (h ^ bounds.map_or(0, |b| b.tag()), i)
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
        /// Quality code per vertex's texture coordinates (`UvQuality::code`).
        quality: Vec<u8>,
        /// Stage that settled each vertex's texture coordinates (`UvStage::code`).
        stage: Vec<u8>,
        /// Evidence strength per vertex (unit of its stage).
        confidence: Vec<f32>,
    },
}

/// `[exact, recovered, unreliable]` vertex counts of a cached decoding.
#[must_use]
pub fn quality_counts(cached: &Cached) -> Option<[usize; 3]> {
    let Cached::Decoded { quality, .. } = cached else {
        return None;
    };
    let mut c = [0usize; 3];
    for q in quality {
        c[usize::from(*q).min(2)] += 1;
    }
    Some(c)
}

fn fnv(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The cache directory of the decoded-geometry cache (the setting *Clear texture cache* and
/// the size shown on the Content page use it).
#[must_use]
pub fn directory() -> Option<PathBuf> {
    dir()
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
    let mut at2 = at + 4 + faces;
    let quality = data.get(at2..at2 + n)?.to_vec();
    at2 += n;
    let stage = data.get(at2..at2 + n)?.to_vec();
    at2 += n;
    let confidence = data
        .get(at2..at2 + n * 4)?
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    Some(Cached::Decoded {
        positions: pos.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect(),
        uvs: uv.chunks_exact(2).map(|c| [c[0], c[1]]).collect(),
        kept,
        quality,
        stage,
        confidence,
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
            quality,
            stage,
            confidence,
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
            out.extend_from_slice(quality);
            out.extend_from_slice(stage);
            for c in confidence {
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, out);
}

/// Captures the result of decoding `decoded` from the original `original` mesh.
#[must_use]
pub fn capture(
    original: &O3d,
    decoded: &O3d,
    report: &opts_core::formats::o3d::descramble::ConfidentReport,
) -> Cached {
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
        quality: report.quality.iter().map(|q| q.code()).collect(),
        stage: report.stages.iter().map(|s| s.code()).collect(),
        confidence: report.confidence.clone(),
    }
}

/// Applies a cached decoding to `o`; returns `false` if it does not fit the mesh.
pub fn apply(o: &mut O3d, cached: &Cached) -> bool {
    let Cached::Decoded {
        positions,
        uvs,
        kept,
        ..
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
    jobs: Sender<Job>,
    state: Arc<Mutex<PoolState>>,
    params: DecodeParams,
}

/// A queued mesh and the files that may hold other scramblings of the same model.
type Job = (
    Key,
    Box<O3d>,
    Vec<std::path::PathBuf>,
    Option<ExpectedBounds>,
);

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
        let (tx, rx) = channel::<Job>();
        let rx = Arc::new(Mutex::new(rx));
        let state = Arc::new(Mutex::new(PoolState::default()));
        let fingerprint = params.fingerprint();
        for i in 0..threads.max(1) {
            let (rx, state, params) = (rx.clone(), state.clone(), params.clone());
            std::thread::Builder::new()
                .name(format!("opts-descramble-{i}"))
                .spawn(move || loop {
                    let job = rx.lock().ok().and_then(|r| r.recv().ok());
                    let Some((key, model, candidates, bounds)) = job else {
                        return;
                    };
                    let t = std::time::Instant::now();
                    let outcomes = decode_with_copies(key, *model, &candidates, bounds, &params);
                    tracing::info!(
                        copies = outcomes.len(),
                        accepted = matches!(outcomes.first(), Some((_, Cached::Decoded { .. }))),
                        ms = t.elapsed().as_millis() as u64,
                        "scrambled mesh decoded (background)"
                    );
                    for (k, outcome) in &outcomes {
                        store(*k, fingerprint, outcome);
                    }
                    if let Ok(mut st) = state.lock() {
                        st.pending.remove(&key);
                        for (k, outcome) in outcomes {
                            st.results.insert(k, outcome);
                        }
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
    pub fn submit(
        &self,
        key: Key,
        model: &O3d,
        candidates: Vec<std::path::PathBuf>,
        bounds: Option<ExpectedBounds>,
    ) {
        let Ok(mut st) = self.state.lock() else {
            return;
        };
        if st.pending.contains(&key) || st.results.contains_key(&key) {
            return;
        }
        st.pending.insert(key);
        drop(st);
        let _ = self
            .jobs
            .send((key, Box::new(model.clone()), candidates, bounds));
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

/// Most further scramblings of one model used for a joint decoding.
const MAX_SIBLINGS: usize = 3;

/// Decodes `model` (file key `key`); when `candidates` hold other scramblings of the same model
/// (different key, same geometry) they are decoded jointly, which resolves more vertices. Returns
/// the outcome of every file decoded, the first being `model`'s.
fn decode_with_copies(
    key: Key,
    model: O3d,
    candidates: &[std::path::PathBuf],
    bounds: Option<ExpectedBounds>,
    params: &DecodeParams,
) -> Vec<(Key, Cached)> {
    let mut copies = vec![model.clone()];
    let mut keys = vec![key];
    for path in candidates {
        if copies.len() > MAX_SIBLINGS {
            break;
        }
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let k = key_bounded(&bytes, 0, bounds);
        if keys.contains(&k) {
            continue;
        }
        let Ok(o) = opts_core::formats::o3d::parse(&bytes) else {
            continue;
        };
        if o.encryption_key.is_some() && same_model(&model, &o) {
            keys.push(k);
            copies.push(o);
        }
    }
    if copies.len() >= 2 {
        let before = copies.clone();
        let reports = decode_joint_bounded(&mut copies, params, bounds);
        if reports.first().is_some_and(|r| r.accepted) {
            return keys
                .into_iter()
                .zip(before.iter().zip(&copies).zip(&reports))
                .map(|(k, ((b, d), r))| {
                    (
                        k,
                        if r.accepted {
                            capture(b, d, r)
                        } else {
                            Cached::Rejected
                        },
                    )
                })
                .collect();
        }
    }
    let mut single = model.clone();
    let report = match bounds {
        Some(b) => decode_confident_bounded(&mut single, params, b),
        None => decode_confident_with(&mut single, params),
    };
    let outcome = if report.accepted {
        capture(&model, &single, &report)
    } else {
        Cached::Rejected
    };
    vec![(key, outcome)]
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
            quality: vec![0, 2],
            stage: vec![2, 6],
            confidence: vec![12.0, 0.5],
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
        pool.submit(k, &mesh, Vec::new(), None);
        pool.submit(k, &mesh, Vec::new(), None); // duplicate submissions are ignored
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
