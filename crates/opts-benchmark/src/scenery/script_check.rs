//! Phase C: the script-effect check (`docs/src/architecture/scenery-benchmark.md`, Script-effect
//! check).
//!
//! This module holds the check's vocabulary: the scenarios, the thresholds, the record block and
//! the tags every scripted object carries. [`super::script_run`] drives the scenery runtime
//! through the scenarios and captures the states; [`super::script_judge`] turns the states into
//! `S-*` categories.

use std::time::Instant;

use super::expected::{Expected, Placement};
use super::metrics::Matte;
use super::preflight::Preflight;
use super::worker::Harness;
use super::{script_judge, script_run, Record, Verdict};

mod record;
mod thresholds;
pub use record::ScriptCheck;
pub use thresholds::ScriptThresholds;

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

/// The views of the script scenarios: the usual oblique view V1 (unless the object has a
/// directional light, then V1 looks along the first such light's shining direction: the
/// runtime hides a directional light outside its cone, as seen from the first capture camera),
/// the top view V2 and a front view V3 (8° above the horizon, square on to the object's front:
/// signal lamps and displays face forward and sit inside visors that hide them from the
/// oblique view).
#[must_use]
pub fn views(
    bounds: &super::framing::Bounds,
    heading_deg: f32,
    pre: &Preflight,
    base: &[super::framing::View; 2],
) -> Vec<super::framing::View> {
    let light = pre.object.as_ref().and_then(|o| {
        o.lights
            .iter()
            .find(|l| !l.omni && l.direction.length() > 0.1)
    });
    let v1 = match light {
        Some(l) => {
            let d = l.direction.normalize();
            let azimuth = heading_deg + d.x.atan2(d.y).to_degrees();
            let elevation = d.z.clamp(-1.0, 1.0).asin().to_degrees().clamp(3.0, 60.0);
            super::framing::view_from(bounds, azimuth, elevation)
        }
        None => base[0],
    };
    vec![
        v1,
        base[1],
        super::framing::view_from(bounds, heading_deg, 8.0),
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
mod tests;
