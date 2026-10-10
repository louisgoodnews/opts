//! On-disk cache of descrambled meshes.
//!
//! Decoding a keyed `.o3d` takes tens to thousands of milliseconds, so the outcome (accepted
//! geometry or a rejection) is stored per file content under the user's cache directory and
//! reused by later sessions. The cache only holds data derived from the user's own installation
//! and never leaves their machine.

use std::path::PathBuf;

use opts_core::formats::o3d::O3d;

/// Bump when the decoder changes in a way that alters its output.
const VERSION: u8 = 1;
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

fn file_for(file_bytes: &[u8], model_index: usize) -> Option<PathBuf> {
    Some(dir()?.join(format!("{:016x}-{model_index}.bin", fnv(file_bytes))))
}

/// Reads the cached outcome for model `model_index` of the file with these bytes.
#[must_use]
pub fn load(file_bytes: &[u8], model_index: usize) -> Option<Cached> {
    let data = std::fs::read(file_for(file_bytes, model_index)?).ok()?;
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
pub fn store(file_bytes: &[u8], model_index: usize, value: &Cached) {
    let Some(path) = file_for(file_bytes, model_index) else {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_decoded_outcome() {
        let _g = std::env::temp_dir().join(format!("opts-cache-test-{}", std::process::id()));
        std::env::set_var("XDG_CACHE_HOME", &_g);
        let value = Cached::Decoded {
            positions: vec![[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]],
            uvs: vec![[0.25, 0.5], [0.75, 1.0]],
            kept: vec![true, false, true],
        };
        store(b"file-a", 0, &value);
        store(b"file-b", 1, &Cached::Rejected);
        assert_eq!(load(b"file-a", 0), Some(value));
        assert_eq!(load(b"file-b", 1), Some(Cached::Rejected));
        assert_eq!(load(b"file-a", 1), None);
        let _ = std::fs::remove_dir_all(_g);
    }
}
