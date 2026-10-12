//! Phase C extension point: the script-effect check
//! (`docs/src/architecture/scenery-benchmark.md`, Script-effect check).
//!
//! Scenery scripts do not run in OPTS yet (Q-SB-11), so every scripted object reports
//! `S-NOT-RUN` and its effect channels as tags. When a scenery-script runtime exists, Phase C
//! adds a function here that takes the worker's [`Harness`](super::worker::Harness), runs each
//! [`Scenario`] with a fixed time step (advancing frames with `Harness::update`), captures the
//! views at each state with `Harness::capture` and compares them with the expected silhouette
//! recomputed from the script variables.

use super::preflight::Preflight;

/// Stimulus scenarios S0–S5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    /// 12:00, day 180, 2 s.
    S0Baseline,
    /// S0 + 10 s.
    S1Later,
    /// 23:00.
    S2Night,
    /// Day 15 (winter).
    S3Winter,
    /// `TrafficLightPhase` 0 → 1 → 2 → 3.
    S4TrafficLight,
    /// Each `[mouseevent]` trigger fired once.
    S5MouseEvents,
}

/// All scenarios in run order.
pub const SCENARIOS: [Scenario; 6] = [
    Scenario::S0Baseline,
    Scenario::S1Later,
    Scenario::S2Night,
    Scenario::S3Winter,
    Scenario::S4TrafficLight,
    Scenario::S5MouseEvents,
];

/// Keywords through which a script shows its effect.
const CHANNELS: &[&str] = &[
    "visible",
    "newanim",
    "light_enh",
    "light_enh_2",
    "matl_change",
    "alphascale",
    "ctc",
    "particle_emitter",
    "smoke",
    "trafficlight",
    "mouseevent",
    "usetexttexture",
    "sound",
];

/// Script status and effect channels of an object; `(None, [])` for objects without a script.
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
    (Some("S-NOT-RUN".into()), channels)
}
