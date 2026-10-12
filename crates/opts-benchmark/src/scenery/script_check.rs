//! Phase C: the script-effect check (`docs/src/architecture/scenery-benchmark.md`, Script-effect
//! check).
//!
//! This module holds the check's vocabulary: the scenarios, the thresholds, the record block and
//! the tags every scripted object carries. [`super::script_run`] drives the scenery runtime
//! through the scenarios and captures the states; [`super::script_judge`] turns the states into
//! `S-*` categories.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::expected::{Expected, Placement};
use super::metrics::Matte;
use super::preflight::Preflight;
use super::worker::Harness;
use super::{script_judge, script_run, Record, Verdict};

/// Stimulus scenarios (S5, the `[mouseevent]` triggers, is not run: Q-SR-07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// 12:00, day 180, 2 s; S1 continues it for 10 s more.
    S0Baseline,
    /// 23:00.
    S2Night,
    /// 15 January (winter).
    S3Winter,
    /// `TrafficLightPhase` 0, 3, 6, 9, 12 (red, red+yellow, green, yellow, off).
    S4TrafficLight,
    /// Other scenery inputs the script reads (`Switch`, `Signal`, …) set to 0, 1, 2.
    S6Inputs,
    /// S0 again on a fresh placement (determinism).
    S0Repeat,
}

/// Keywords through which a script shows its effect.
const CHANNELS: &[&str] = &[
    "visible",
    "newanim",
    "light_enh",
    "light_enh_2",
    "matl_lightmap",
    "matl_change",
    "alphascale",
    "ctc",
    "particle_emitter",
    "smoke",
    "trafficlight",
    "mouseevent",
    "usetexttexture",
    "texttexture_enh",
    "sound",
];

/// Keywords the runtime does not turn into a visible effect (reported, never judged).
const UNSUPPORTED_CHANNELS: &[&str] = &[
    "matl_change",
    "alphascale",
    "ctc",
    "particle_emitter",
    "smoke",
    "mouseevent",
    "texttexture_enh",
];

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

/// The record block of the script check.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScriptCheck {
    /// `ok`, `no-channel`, `unobservable` or the first `S-*` category.
    pub status: String,
    /// Captured states (`s0`, `s1`, `s2`, `s3`, `s4_0` …, `s0b`).
    pub states: Vec<String>,
    /// Effect channels with at least one element the object binds.
    pub channels: Vec<String>,
    /// Stimuli judged (mesh, animation and light-map elements whose value differs between states).
    pub stimuli: usize,
    /// Of those, stimuli whose pixels changed.
    pub live: usize,
    /// Light stimuli judged.
    pub light_stimuli: usize,
    /// Of those, lights that showed.
    pub light_live: usize,
    /// Stimuli that changed nothing (`visible:2`, `anim:0`, `light:1` …).
    pub dead: Vec<String>,
    /// Consistency failures (`s2 v1 leak mesh 3`, `s0 v2 missing`, …).
    pub mismatches: Vec<String>,
    /// Script variables whose value differs between states (first 40).
    pub changed_vars: Vec<String>,
    /// Script diagnostics (messages of severity error, run-time messages, missing files).
    pub errors: Vec<String>,
    /// Channels the runtime or the check does not support (`[mouseevent]`, …).
    pub unsupported: Vec<String>,
    /// Share of pixels that differ between the two runs of S0.
    pub nondeterministic: Option<f32>,
    /// Wall-clock milliseconds of the whole check.
    pub ms: u64,
}

/// Script status and effect channels of an object; `(None, [])` for objects without a script.
/// Without `--scripts` a scripted object is `not-checked`.
#[must_use]
pub fn evaluate(pre: &Preflight) -> (Option<String>, Vec<String>) {
    let scripted = pre.object.as_ref().is_some_and(|o| !o.scripts.is_empty());
    if !scripted {
        return (None, Vec::new());
    }
    let channels = CHANNELS
        .iter()
        .filter(|k| pre.keywords.contains(**k))
        .map(|k| format!("[{k}]"))
        .collect();
    (Some("not-checked".into()), channels)
}

/// The unsupported channels the object uses.
#[must_use]
pub fn unsupported_channels(pre: &Preflight) -> Vec<String> {
    UNSUPPORTED_CHANNELS
        .iter()
        .filter(|k| pre.keywords.contains(**k))
        .map(|k| format!("[{k}]"))
        .collect()
}

/// The views of the script scenarios: V1 is the usual oblique view, unless the object has a
/// directional light; then V1 looks along the first such light's shining direction (the
/// runtime hides a directional light outside its cone, as seen from the first capture camera).
#[must_use]
pub fn views(
    bounds: &super::framing::Bounds,
    heading_deg: f32,
    pre: &Preflight,
    base: &[super::framing::View; 2],
) -> [super::framing::View; 2] {
    let light = pre.object.as_ref().and_then(|o| {
        o.lights
            .iter()
            .find(|l| !l.omni && l.direction.length() > 0.1)
    });
    let Some(l) = light else {
        return *base;
    };
    let d = l.direction.normalize();
    let azimuth = heading_deg + d.x.atan2(d.y).to_degrees();
    let elevation = d.z.clamp(-1.0, 1.0).asin().to_degrees().clamp(3.0, 60.0);
    [
        super::framing::view_from(bounds, azimuth, elevation),
        base[1],
    ]
}

/// Whether the object has `[trafficlight]` (scenario S4 applies).
#[must_use]
pub fn has_traffic_light(pre: &Preflight) -> bool {
    pre.keywords.contains("trafficlight")
}

/// Whether the object has a script.
#[must_use]
pub fn is_scripted(pre: &Preflight) -> bool {
    pre.object.as_ref().is_some_and(|o| !o.scripts.is_empty())
}

/// Runs the scenarios and the judgement for an object that loaded and rendered, and folds the
/// result into `record` (verdict, categories, `script_check`, `tags.script`). Returns the
/// captured mattes for `--keep-images`.
pub fn stage(
    h: &mut Harness,
    ctx: &script_run::Ctx<'_>,
    exp: &Expected,
    place: &Placement,
    record: &mut Record,
) -> Vec<(String, Matte)> {
    let started = Instant::now();
    let outcome = match script_run::run(h, ctx) {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(sco = %ctx.entry.sco, "script scenarios aborted: {e:#}");
            record.tags.script = Some("aborted".into());
            record.tags.info.push("I-SCRIPT-ABORTED".into());
            return Vec::new();
        }
    };
    let Some(sco) = ctx.pre.object.as_ref() else {
        return Vec::new();
    };
    let mut j = script_judge::judge(&outcome, sco, exp, place, ctx.views, ctx.thresholds);
    j.check.unsupported = unsupported_channels(ctx.pre);
    j.check.ms = started.elapsed().as_millis() as u64;
    record.tags.script = Some(j.check.status.clone());
    for c in &j.categories {
        if !record.categories.contains(c) {
            record.categories.push(c.clone());
        }
    }
    if !j.categories.is_empty() {
        record.verdict = Verdict::Fail;
    } else if record.verdict == Verdict::Pass && j.check.status != "ok" {
        record.verdict = Verdict::Unknown;
        record.unknown_reason = Some(format!("script: {}", j.check.status));
    }
    record.script_check = Some(j.check);
    j.shots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_round_trip_and_default_when_missing() {
        let t = ScriptThresholds::default();
        let json = serde_json::to_string(&t).unwrap_or_default();
        let back: ScriptThresholds = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(t, back);
        let partial: ScriptThresholds = serde_json::from_str("{}").unwrap_or_default();
        assert_eq!(partial, t);
    }
}
