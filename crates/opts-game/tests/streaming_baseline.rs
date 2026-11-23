//! The deterministic baseline of the hybrid streamer (M-0114): fixed camera paths and the tile
//! load and unload counts they produce. A change of the policy that moves a count fails here;
//! the table is then re-recorded on purpose. Timing baselines need game runs and are not here.

#[path = "streaming_baseline/replay.rs"]
mod replay;

use opts_game::in_game::stream_budget::policy::StreamMode;
use replay::{replay, Outcome, Segment};

const KMH_30: f32 = 30.0 / 3.6;
const KMH_90: f32 = 90.0 / 3.6;

/// Two minutes of standing still.
const STOP: &[Segment] = &[Segment {
    jump_m: 0.0,
    speed_mps: 0.0,
    frames: 7200,
}];
/// Two minutes at 30 km/h east.
const DRIVE_30: &[Segment] = &[Segment {
    jump_m: 0.0,
    speed_mps: KMH_30,
    frames: 7200,
}];
/// Two minutes at 90 km/h east.
const DRIVE_90: &[Segment] = &[Segment {
    jump_m: 0.0,
    speed_mps: KMH_90,
    frames: 7200,
}];
/// A minute standing, a 5 km teleport, a minute standing.
const TELEPORT: &[Segment] = &[
    Segment {
        jump_m: 0.0,
        speed_mps: 0.0,
        frames: 3600,
    },
    Segment {
        jump_m: 5000.0,
        speed_mps: 0.0,
        frames: 3600,
    },
];

/// The table: scenario, `streaming.mode`, the path and the expected counts.
const TABLE: &[(&str, StreamMode, &[Segment], Outcome)] = &[
    ("stop 120 s", StreamMode::Auto, STOP, ROW_STOP),
    ("drive 30 km/h 120 s", StreamMode::Auto, DRIVE_30, ROW_30),
    ("drive 90 km/h 120 s", StreamMode::Auto, DRIVE_90, ROW_90),
    ("teleport 5 km", StreamMode::Auto, TELEPORT, ROW_TELEPORT),
    (
        "drive 90 km/h forced tile",
        StreamMode::Tile,
        DRIVE_90,
        ROW_90_TILE,
    ),
    (
        "drive 90 km/h forced object",
        StreamMode::Object,
        DRIVE_90,
        ROW_90_OBJECT,
    ),
];

// Recorded from the policy as it stands on agent/streaming (master d3fd638e, evict_keys moved out
// of hybrid.rs without a change of behaviour). The drive rows churn: unloads are nearly as many as
// loads (about one tile in and one out per frame), which the drive rows show but the cause is not
// yet isolated.
const ROW_STOP: Outcome = Outcome {
    loads: 25,
    unloads: 0,
    resident_end: 25,
    tile_frames: 0,
};
const ROW_30: Outcome = Outcome {
    loads: 3631,
    unloads: 3599,
    resident_end: 32,
    tile_frames: 7172,
};
const ROW_90: Outcome = Outcome {
    loads: 6031,
    unloads: 5999,
    resident_end: 32,
    tile_frames: 7195,
};
const ROW_TELEPORT: Outcome = Outcome {
    loads: 50,
    unloads: 25,
    resident_end: 25,
    tile_frames: 0,
};
const ROW_90_TILE: Outcome = Outcome {
    loads: 6031,
    unloads: 5999,
    resident_end: 32,
    tile_frames: 7200,
};
const ROW_90_OBJECT: Outcome = Outcome {
    loads: 51,
    unloads: 25,
    resident_end: 26,
    tile_frames: 0,
};

#[test]
fn replay_counts_match_the_recorded_table() {
    let differing: Vec<String> = TABLE
        .iter()
        .filter_map(|(name, mode, path, expected)| {
            let got = replay(*mode, path);
            (got != *expected).then(|| format!("{name} ({}): {got:?}", mode.name()))
        })
        .collect();
    assert!(
        differing.is_empty(),
        "table differs:\n{}",
        differing.join("\n")
    );
}

#[test]
fn resident_tiles_are_loads_minus_unloads() {
    for (name, mode, path, _) in TABLE {
        let got = replay(*mode, path);
        assert_eq!(
            got.resident_end as i64,
            i64::from(got.loads) - i64::from(got.unloads),
            "scenario `{name}`"
        );
    }
}
