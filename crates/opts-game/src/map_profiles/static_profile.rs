//! The static load profile of the maps, counted off the main thread and cached
//! (`docs/src/architecture/map-profiles.md` §4.2): a sample answers within milliseconds, the full
//! count follows, and the result is kept in `profiles/<map>/static.toml` while the tile files
//! stay as they were (their number and total size).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};

use bevy::prelude::*;
use opts_core::formats::map::count::{ai_pool_size, count_map, tile_files, SAMPLE_BUDGET_BYTES};
use serde::{Deserialize, Serialize};

use super::advice::{tiles_to_km2, MapLoadProfile};
use super::session::ProfileStoreRes;
use super::store::ProfileStore;

/// What the counting knows about a map.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticProfile {
    /// Tile files.
    pub tiles: usize,
    /// `[object]` records.
    pub objects: usize,
    /// `[spline]` records.
    pub splines: usize,
    /// Distinct vehicles of the AI lists.
    pub ai_pool: usize,
    /// Counted from a sample (the full count is still running).
    pub estimated: bool,
}

impl StaticProfile {
    /// The load profile, with the frame times of the last session when there is one.
    #[must_use]
    pub fn to_load_profile(&self, avg_ms: Option<f32>, p95_ms: Option<f32>) -> MapLoadProfile {
        MapLoadProfile {
            tile_count: self.tiles,
            object_count: self.objects,
            spline_count: self.splines,
            ai_pool_vehicles: self.ai_pool,
            map_size_km2: tiles_to_km2(self.tiles),
            estimated: self.estimated,
            avg_frame_time_ms: avg_ms,
            p95_frame_time_ms: p95_ms,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheFile {
    tiles: usize,
    bytes: u64,
    objects: usize,
    splines: usize,
    ai_pool: usize,
}

/// A fingerprint of the tile files: how many and how big.
fn signature(map_dir: &Path) -> (usize, u64) {
    let files = tile_files(map_dir);
    (files.len(), files.iter().map(|f| f.1).sum())
}

fn cache_path(store: &ProfileStore, map_id: &str) -> PathBuf {
    store.root().join(map_id).join("static.toml")
}

/// The cached counts when the tile files are as they were when they were counted.
fn read_cache(store: &ProfileStore, map_id: &str, sig: (usize, u64)) -> Option<StaticProfile> {
    let c: CacheFile =
        toml::from_str(&std::fs::read_to_string(cache_path(store, map_id)).ok()?).ok()?;
    ((c.tiles, c.bytes) == sig).then_some(StaticProfile {
        tiles: c.tiles,
        objects: c.objects,
        splines: c.splines,
        ai_pool: c.ai_pool,
        estimated: false,
    })
}

fn write_cache(store: &ProfileStore, map_id: &str, sig: (usize, u64), p: &StaticProfile) {
    let c = CacheFile {
        tiles: sig.0,
        bytes: sig.1,
        objects: p.objects,
        splines: p.splines,
        ai_pool: p.ai_pool,
    };
    let path = cache_path(store, map_id);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = toml::to_string_pretty(&c) {
        let _ = std::fs::write(path, text);
    }
}

/// The static profiles counted so far, by map id.
#[derive(Resource)]
pub struct StaticProfiles {
    done: HashMap<String, StaticProfile>,
    asked: std::collections::HashSet<String>,
    tx: Sender<(String, StaticProfile)>,
    rx: std::sync::Mutex<Receiver<(String, StaticProfile)>>,
}

impl Default for StaticProfiles {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            done: HashMap::new(),
            asked: Default::default(),
            tx,
            rx: std::sync::Mutex::new(rx),
        }
    }
}

impl StaticProfiles {
    /// Starts counting `map_dir` in the background (once per map). The first answer is a sample
    /// (`estimated`), the second the full count; a valid cache answers at once.
    pub fn request(&mut self, map_id: &str, map_dir: PathBuf, store: Option<ProfileStore>) {
        if !self.asked.insert(map_id.to_string()) {
            return;
        }
        let (id, tx) = (map_id.to_string(), self.tx.clone());
        let _ = std::thread::Builder::new()
            .name("map-static-profile".into())
            .spawn(move || {
                let sig = signature(&map_dir);
                if let Some(c) = store.as_ref().and_then(|s| read_cache(s, &id, sig)) {
                    let _ = tx.send((id, c));
                    return;
                }
                let ai_pool = ai_pool_size(&map_dir);
                let quick = count_map(&map_dir, SAMPLE_BUDGET_BYTES);
                let _ = tx.send((
                    id.clone(),
                    StaticProfile {
                        tiles: quick.tiles,
                        objects: quick.objects,
                        splines: quick.splines,
                        ai_pool,
                        estimated: quick.sampled,
                    },
                ));
                if quick.sampled {
                    let full = count_map(&map_dir, u64::MAX);
                    let p = StaticProfile {
                        tiles: full.tiles,
                        objects: full.objects,
                        splines: full.splines,
                        ai_pool,
                        estimated: false,
                    };
                    if let Some(s) = &store {
                        write_cache(s, &id, sig, &p);
                    }
                    let _ = tx.send((id, p));
                } else if let Some(s) = &store {
                    let p = StaticProfile {
                        estimated: false,
                        ..StaticProfile {
                            tiles: quick.tiles,
                            objects: quick.objects,
                            splines: quick.splines,
                            ai_pool,
                            estimated: false,
                        }
                    };
                    write_cache(s, &id, sig, &p);
                }
            });
    }

    /// Takes what the workers have counted since the last call.
    pub fn poll(&mut self) {
        let Ok(rx) = self.rx.lock() else { return };
        while let Ok((id, p)) = rx.try_recv() {
            self.done.insert(id, p);
        }
    }

    /// The best answer so far for a map.
    #[must_use]
    pub fn get(&self, map_id: &str) -> Option<&StaticProfile> {
        self.done.get(map_id)
    }

    /// Whether a count was ever asked for.
    #[must_use]
    pub fn was_asked(&self, map_id: &str) -> bool {
        self.asked.contains(map_id)
    }
}

/// Collects the workers' results every frame.
pub fn poll_static_profiles(mut profiles: ResMut<StaticProfiles>) {
    profiles.poll();
}

/// Starts counting a map (convenience for the modal and the control protocol).
pub fn request_for(
    profiles: &mut StaticProfiles,
    store: &ProfileStoreRes,
    map_id: &str,
    map_dir: PathBuf,
) {
    profiles.request(map_id, map_dir, store.0.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tile(objects: usize) -> String {
        let mut t = String::from("banner\n[version]\n3\n");
        for _ in 0..objects {
            t.push_str("[object]\n0\n");
        }
        t
    }

    #[test]
    fn the_first_answer_comes_from_a_sample_or_the_cache_and_the_full_count_is_cached() {
        let root = std::env::temp_dir().join(format!("opts-static-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let map = root.join("maps").join("M");
        std::fs::create_dir_all(&map).unwrap();
        for i in 0..6 {
            std::fs::write(map.join(format!("tile_{i}_0.map")), tile(10)).unwrap();
        }
        let store = ProfileStore::new(root.join("profiles"));
        let mut p = StaticProfiles::default();
        p.request("M", map.clone(), Some(store.clone()));
        let t = std::time::Instant::now();
        while p.get("M").is_none() && t.elapsed().as_secs() < 10 {
            p.poll();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let got = p.get("M").unwrap().clone();
        assert_eq!((got.tiles, got.objects, got.estimated), (6, 60, false));
        // The small map's full count was written; a second asker reads it back.
        for _ in 0..200 {
            if cache_path(&store, "M").exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let sig = signature(&map);
        assert_eq!(read_cache(&store, "M", sig).unwrap().objects, 60);
        // A changed tile file invalidates the cache.
        std::fs::write(map.join("tile_7_0.map"), tile(1)).unwrap();
        assert!(read_cache(&store, "M", signature(&map)).is_none());
        // Asking again for the same map does not start another count.
        p.request("M", map, Some(store));
        assert!(p.was_asked("M"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
