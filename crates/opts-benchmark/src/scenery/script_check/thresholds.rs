//! Thresholds of the script-effect check, written into `run.json`.

use serde::{Deserialize, Serialize};

/// Thresholds of the script check (written into `run.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScriptThresholds {
    /// Step of `advance_scenery` (seconds).
    pub dt_s: f32,
    /// Changed share of the whole silhouette that makes a `lightmap` stimulus live.
    pub live_changed_share: f32,
    /// A `visible` or `anim` stimulus is live when at least this share of the mesh's own area
    /// (in pixels) changed inside its region …
    pub live_area_frac: f32,
    /// … and at least this many pixels.
    pub live_min_px: usize,
    /// Share of stimuli that must be live.
    pub live_fraction: f32,
    /// A hidden mesh leaks when this share of its own area is drawn.
    pub leak_share: f32,
    /// A leak needs this many pixels of own area.
    pub leak_min_px: usize,
    /// A light must brighten its window by this share …
    pub light_gain: f32,
    /// … but at least this much (0..255).
    pub light_gain_abs: f32,
    /// A pixel counts as changed when a channel differs by this much (0..255).
    pub delta: u8,
    /// Most pixels that may differ between the two runs of S0 (share).
    pub det_max_diff: f32,
    /// A mesh region needs this many pixels in a view to be judged there.
    pub min_region_px: usize,
}

impl Default for ScriptThresholds {
    fn default() -> Self {
        Self {
            dt_s: 1.0 / 60.0,
            live_changed_share: 0.02,
            live_area_frac: 0.15,
            live_min_px: 4,
            live_fraction: 0.5,
            leak_share: 0.5,
            leak_min_px: 30,
            light_gain: 0.10,
            light_gain_abs: 8.0,
            delta: 16,
            det_max_diff: 0.001,
            min_region_px: 10,
        }
    }
}
