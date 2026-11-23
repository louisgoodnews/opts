//! The load profile of a map and the heuristic engine that turns it into advice
//! (`docs/src/architecture/map-profiles.md` §4).
//!
//! The thresholds are calibrated on the maps of the installation the design was measured on
//! (47 maps: median 40,985 objects, 75th percentile 100,107, 90th percentile 242,118); the
//! task's 50,000 stays as the first note. They are named constants so the calibration is one
//! place to change.

/// More objects than this is above the typical map (the task's line).
pub const OBJECTS_NOTE: usize = 50_000;
/// More objects than this is a high geometry load (about the 80th percentile).
pub const OBJECTS_HIGH: usize = 150_000;
/// Fewer objects than this is a light map (about the 30th percentile).
pub const OBJECTS_LIGHT: usize = 20_000;
/// More splines than this is a lot of road geometry.
pub const SPLINES_NOTE: usize = 40_000;
/// An AI pool larger than this with the traffic setting above [`TRAFFIC_NOTE`] is a high AI load.
pub const AI_POOL_HIGH: usize = 200;
/// The traffic setting above which a large pool matters.
pub const TRAFFIC_NOTE: i64 = 100;
/// An average frame time above this (ms, 30 fps) is slow.
pub const SLOW_MS: f32 = 33.0;
/// An average frame time at or below this (ms, 60 fps) is smooth.
pub const SMOOTH_MS: f32 = 16.7;

/// What the profiler knows about a map before it is loaded.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapLoadProfile {
    /// Tiles of the map.
    pub tile_count: usize,
    /// `[object]` records over all tiles.
    pub object_count: usize,
    /// `[spline]` and `[spline_h]` records.
    pub spline_count: usize,
    /// Distinct vehicle entries in the map's AI lists (the pool, not the number on the road).
    pub ai_pool_vehicles: usize,
    /// Area in km² (a tile is 300 m × 300 m).
    pub map_size_km2: f32,
    /// Counted from a sample of tiles.
    pub estimated: bool,
    /// Average frame time of the last session on this computer (ms).
    pub avg_frame_time_ms: Option<f32>,
    /// 95th-percentile frame time of that session (ms).
    pub p95_frame_time_ms: Option<f32>,
}

/// Area of `tiles` tiles in km².
#[must_use]
pub fn tiles_to_km2(tiles: usize) -> f32 {
    tiles as f32 * 0.09
}

/// How serious a piece of advice is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Good news or a remark.
    Info,
    /// Worth acting on.
    Warning,
}

/// Which advice (also the message key suffix).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdviceId {
    /// Above the typical object count.
    GeometryAboveTypical,
    /// A high geometry load.
    GeometryHigh,
    /// A light map.
    LightMap,
    /// A lot of road geometry.
    ManySplines,
    /// A large AI pool with high traffic.
    HighAiLoad,
    /// The map ran slowly last time.
    RanSlowly,
    /// The map ran smoothly last time.
    RanSmoothly,
    /// No session recorded yet.
    NotPlayedYet,
}

/// One recommendation with the settings change it suggests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advice {
    /// Which advice.
    pub id: AdviceId,
    /// How serious.
    pub severity: Severity,
    /// Settings to change, as `(key, stored text)`; empty for a remark.
    pub suggested: Vec<(String, String)>,
    /// The preset that carries the suggestion (`performance`, `balanced`, `quality`).
    pub preset: Option<&'static str>,
}

/// The advice for a map; `traffic` is the current `gameplay.traffic`.
#[must_use]
pub fn advise(p: &MapLoadProfile, traffic: i64) -> Vec<Advice> {
    let pairs = |v: &[(&str, &str)]| -> Vec<(String, String)> {
        v.iter()
            .map(|(k, x)| ((*k).to_string(), (*x).to_string()))
            .collect()
    };
    let mut out = Vec::new();
    if p.object_count > OBJECTS_HIGH {
        out.push(Advice {
            id: AdviceId::GeometryHigh,
            severity: Severity::Warning,
            suggested: pairs(&[
                ("graphics.light_quality_mode", "custom"),
                ("graphics.light_shafts", "low"),
                ("graphics.light_cone_cap", "32"),
                ("gameplay.tile_radius", "2"),
                ("graphics.mirror_count", "1"),
            ]),
            preset: Some("performance"),
        });
    } else if p.object_count > OBJECTS_NOTE {
        out.push(Advice {
            id: AdviceId::GeometryAboveTypical,
            severity: Severity::Info,
            suggested: pairs(&[("graphics.light_shafts", "low")]),
            preset: None,
        });
    } else if p.object_count < OBJECTS_LIGHT && p.object_count > 0 {
        out.push(Advice {
            id: AdviceId::LightMap,
            severity: Severity::Info,
            suggested: Vec::new(),
            preset: Some("quality"),
        });
    }
    if p.spline_count > SPLINES_NOTE {
        out.push(Advice {
            id: AdviceId::ManySplines,
            severity: Severity::Info,
            suggested: Vec::new(),
            preset: None,
        });
    }
    if p.ai_pool_vehicles > AI_POOL_HIGH && traffic > TRAFFIC_NOTE {
        out.push(Advice {
            id: AdviceId::HighAiLoad,
            severity: Severity::Warning,
            suggested: pairs(&[("gameplay.traffic", "60")]),
            preset: None,
        });
    }
    match p.avg_frame_time_ms {
        Some(ms) if ms > SLOW_MS => out.push(Advice {
            id: AdviceId::RanSlowly,
            severity: Severity::Warning,
            suggested: Vec::new(),
            preset: Some("performance"),
        }),
        Some(ms) if ms <= SMOOTH_MS => out.push(Advice {
            id: AdviceId::RanSmoothly,
            severity: Severity::Info,
            suggested: Vec::new(),
            preset: None,
        }),
        Some(_) => {}
        None => out.push(Advice {
            id: AdviceId::NotPlayedYet,
            severity: Severity::Info,
            suggested: Vec::new(),
            preset: None,
        }),
    }
    out
}
