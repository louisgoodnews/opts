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

mod cache;
use cache::{read_cache, signature, write_cache};

#[cfg(test)]
mod tests;
