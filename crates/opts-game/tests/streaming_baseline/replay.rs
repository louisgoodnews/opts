//! Deterministic replay of the hybrid streamer's per-frame plan on a camera path (M-0114
//! baseline). It calls the same pure rules the streamer calls (motion, blend, latch, look-ahead,
//! heading weight, eviction, weighted queue), without Bevy, so the tile load and unload counts of
//! a path are a fixed number. Model: one tile load per frame, a refill of [`REFILL`] candidates
//! when the queue is empty, every tile assumed available.

use std::collections::{BTreeSet, VecDeque};

use opts_game::in_game::stream_budget::policy::{
    effective_blend, heading_weight, latch, lookahead, Regime, StreamMode, REFILL,
};
use opts_game::in_game::stream_budget::{
    evict_keys, wanted_tiles_weighted, Frame, Motion, TileKey,
};

/// Ring radius in tiles of the replay.
pub const RADIUS: i32 = 2;
/// Tile edge in metres of the replay grid.
pub const TILE_M: f32 = 500.0;
/// Frame time of the replay (60 Hz).
pub const FRAME_S: f32 = 1.0 / 60.0;

/// One piece of a camera path: an optional jump (teleport) at its first frame, then a steady
/// speed along the east axis for `frames` frames.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// Jump in metres at the first frame (0 for none).
    pub jump_m: f32,
    /// Speed in m/s.
    pub speed_mps: f32,
    /// Frames of the segment.
    pub frames: u32,
}

/// Counts of one replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// Tiles loaded (a tile reloaded after an unload counts again).
    pub loads: u32,
    /// Tiles unloaded.
    pub unloads: u32,
    /// Tiles resident at the end.
    pub resident_end: usize,
    /// Frames in the tile regime.
    pub tile_frames: u32,
}

/// Replays `segments` under `mode` and counts the loads and unloads.
#[must_use]
pub fn replay(mode: StreamMode, segments: &[Segment]) -> Outcome {
    let mut motion = Motion::default();
    let mut regime = Regime::default();
    let mut resident: BTreeSet<TileKey> = BTreeSet::new();
    let mut queue: VecDeque<TileKey> = VecDeque::new();
    let (mut loads, mut unloads, mut tile_frames, mut x) = (0u32, 0u32, 0u32, 0.0f32);
    for seg in segments {
        for frame in 0..seg.frames {
            if frame == 0 {
                x += seg.jump_m;
            }
            x += seg.speed_mps * FRAME_S;
            let track = (x, 0.0);
            motion.observe(track, FRAME_S);
            let blend = effective_blend(mode, motion.speed_mps());
            regime = latch(regime, blend);
            tile_frames += u32::from(regime == Regime::Tile);
            let ahead = lookahead(blend);
            let centre = ((track.0 / TILE_M).floor() as i32, 0);
            let held: Vec<TileKey> = resident.iter().copied().collect();
            for key in evict_keys(&held, centre, RADIUS, ahead) {
                if resident.remove(&key) {
                    unloads += 1;
                }
            }
            if queue.is_empty() {
                let view = Frame {
                    centre,
                    radius: RADIUS,
                    cam: (track.0 / TILE_M, 0.0),
                    forward: motion.heading(),
                    ahead,
                    weight: heading_weight(blend),
                };
                let snapshot = resident.clone();
                let keys = wanted_tiles_weighted(view, |k| snapshot.contains(&k), |_| true);
                queue.extend(keys.into_iter().take(REFILL));
            }
            if let Some(key) = queue.pop_front() {
                if resident.insert(key) {
                    loads += 1;
                }
            }
        }
    }
    Outcome {
        loads,
        unloads,
        resident_end: resident.len(),
        tile_frames,
    }
}
