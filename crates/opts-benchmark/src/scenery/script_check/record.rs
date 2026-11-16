//! The record block of the script check: status, states, stimuli and diagnostics.

use serde::{Deserialize, Serialize};

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
    /// The bound variables (`[visible]`, `[newanim]`, lights) with their value in each state,
    /// in the order of `states` (at most 12 variables).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub bound: std::collections::BTreeMap<String, Vec<f32>>,
    /// Script variables whose value differs between states (first 40).
    pub changed_vars: Vec<String>,
    /// Script diagnostics (messages of severity error, run-time messages, missing files).
    pub errors: Vec<String>,
    /// Channels the runtime or the check does not support (`[mouseevent]`, …).
    pub unsupported: Vec<String>,
    /// Share of pixels that differ between the two runs of S0.
    pub nondeterministic: Option<f32>,
    /// Script variables whose value differs between the two runs of S0.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nondeterministic_vars: Vec<String>,
    /// Wall-clock milliseconds of the whole check.
    pub ms: u64,
}
