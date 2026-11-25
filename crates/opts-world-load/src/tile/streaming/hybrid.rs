//! The per-frame plan of the hybrid streamer (M-0114): the speed decides the blend, the look-ahead,
//! the heading weight and the regime label; the streamer applies them to the queue and to eviction.

use super::super::*;
use opts_stream_budget::order::near_first_for;
use opts_stream_budget::policy::{
    effective_blend, heading_weight, latch, lookahead, Regime, StreamStatus, REFILL,
};
use opts_stream_budget::{evict_keys, Motion};
use opts_tile_loading::TileMode;

/// What persists between frames for the plan: the motion estimate and the regime label.
#[derive(Debug, Default)]
pub struct HybridState {
    motion: Motion,
    regime: Regime,
}

/// What this frame's speed asks of the streamer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    /// Tile steps beyond the ring along the heading.
    pub ahead: i32,
    /// Heading weight of the queue priority.
    pub weight: f32,
    /// Direction of travel on the ground plane.
    pub heading: (f32, f32),
    /// The regime label after this frame.
    pub regime: Regime,
}

/// Reads the track of the player (`track`, scene metres east/north) for this frame and plans the
/// streaming. Publishes the focus of the object regime and the status to the job.
pub fn plan_frame(
    job: &mut WorldLoadJob,
    state: &mut HybridState,
    track: (f32, f32),
    dt: f32,
) -> Plan {
    state.motion.observe(track, dt);
    let speed = state.motion.speed_mps();
    let blend = effective_blend(job.stream_mode, speed);
    state.regime = latch(state.regime, blend);
    job.stream_focus = near_first_for(blend).then_some(track);
    job.stream_status = StreamStatus {
        regime: state.regime,
        speed_mps: speed,
        blend,
    };
    Plan {
        ahead: lookahead(blend),
        weight: heading_weight(blend),
        heading: state.motion.heading(),
        regime: state.regime,
    }
}

/// Resident tiles to unload: beyond the radius plus the plan's hysteresis, then, if still over
/// the cap, the farthest. Nothing in *load all* mode.
pub fn evictions(job: &WorldLoadJob, centre: (i32, i32), plan: &Plan) -> Vec<(i32, i32)> {
    let TileMode::Radius(r) = job.tile_mode() else {
        return Vec::new();
    };
    let resident: Vec<(i32, i32)> = job.resident.keys().copied().collect();
    evict_keys(&resident, centre, r, plan.ahead)
}

/// Queues the next tiles: in radius mode the ring and the look-ahead, nearest by the heading
/// weighted distance; in *load all* mode the listed tiles nearest first.
pub fn refill(job: &mut WorldLoadJob, centre: (i32, i32), track: (f32, f32), plan: &Plan) {
    let keys = match job.tile_mode() {
        TileMode::All => {
            let skip = |k| job.resident.contains_key(&k) || job.missing_tiles.contains(&k);
            opts_tile_loading::wanted(TileMode::All, centre, job.listed_tiles(), skip)
        }
        TileMode::Radius(r) => {
            let ts = job.grid.tile_size(centre.1).max(1.0) as f32;
            let o = job.tile_offset(centre);
            let cam = (
                centre.0 as f32 + (track.0 - o.0) / ts,
                centre.1 as f32 + (track.1 - o.1) / ts,
            );
            let frame = opts_stream_budget::Frame {
                centre,
                radius: r,
                cam,
                forward: plan.heading,
                ahead: plan.ahead,
                weight: plan.weight,
            };
            let (res, lst, miss) = (&job.resident, &job.tile_files, &job.missing_tiles);
            opts_stream_budget::wanted_tiles_weighted(
                frame,
                |k| res.contains_key(&k),
                |k| lst.contains_key(&k) && !miss.contains(&k),
            )
        }
    };
    let mut queued = 0;
    for key in keys {
        if queued == REFILL {
            break;
        }
        if job.tile_available(key) {
            job.queue.push_back(key);
            queued += 1;
        }
    }
}
