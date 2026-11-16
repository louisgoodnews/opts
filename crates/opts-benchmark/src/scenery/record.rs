//! The record of one object (`results.jsonl`) and the blocks it is made of.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::script_check::ScriptCheck;
use super::{ObjectEntry, Verdict, ViewMetrics};

/// Feature tags of a record.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Tags {
    /// Keywords the object uses that OPTS does not implement for scenery.
    pub unsupported: Vec<String>,
    /// Script-effect status (`not-checked`, `ok`, `no-channel`, `unobservable` or an `S-*` category).
    pub script: Option<String>,
    /// Phase C effect channels found by preflight.
    pub effect_channels: Vec<String>,
    /// Informational categories that do not change the verdict (`F-LOD-OVERDRAW` until M-0029).
    #[serde(default)]
    pub info: Vec<String>,
}

/// One captured `WARN`/`ERROR` event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventRecord {
    /// `WARN` or `ERROR`.
    pub level: String,
    /// Tracing target (module path).
    pub target: String,
    /// Message with the OMSI root replaced by `<root>`.
    pub template: String,
    /// Other fields, shortened.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detail: String,
}

/// Loader counters and events of one object.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Diagnostics {
    /// `WorldStats` deltas while the object loaded.
    pub stats: BTreeMap<String, i64>,
    /// Captured events.
    pub events: Vec<EventRecord>,
}

/// Audio block: recorded, never verified.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Audio {
    /// Always `recorded`.
    pub status: String,
    /// `[sound]` file named by the object.
    pub sound_cfg: Option<String>,
    /// Whether that file exists.
    pub resolves: Option<bool>,
    /// Sound triggers raised by scripts (not collected yet: the runtime drops them).
    pub triggers: Vec<String>,
}

/// Result of the text-texture check.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TextCheck {
    /// Mean colour difference between string sets A and B inside the text regions, per view
    /// (`t1` looks at the text triangles' front, `t2` at their back).
    pub mean_diff: BTreeMap<String, f32>,
    /// Share of text-region pixels whose colour changed by more than the threshold delta
    /// between the two string sets, per view.
    #[serde(default)]
    pub changed_share: BTreeMap<String, f32>,
    /// Text-region pixels per view.
    pub px_text: BTreeMap<String, usize>,
}

/// Wall-clock milliseconds per step.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Timing {
    /// Parsing the object and building the expected geometry.
    pub preflight: u64,
    /// Tile rewrite and rebuild.
    pub load: u64,
    /// Waiting for decoding and the settle frames.
    pub settle: u64,
    /// Off-screen renders and readback.
    pub render: u64,
    /// Rasterising, metrics and verdict.
    pub analyse: u64,
    /// The whole object.
    pub total: u64,
}

/// One line of `results.jsonl`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// Relative `.sco` path.
    pub sco: String,
    /// Object key.
    pub key: String,
    /// Corpus tier.
    pub tier: String,
    /// Context classes.
    pub class: Vec<String>,
    /// Verdict.
    pub verdict: Verdict,
    /// Failure categories (`F-*`).
    pub categories: Vec<String>,
    /// Why the verdict is `unknown`, if it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unknown_reason: Option<String>,
    /// Feature tags.
    pub tags: Tags,
    /// Per-view metrics (`v1`, `v2`).
    pub views: BTreeMap<String, ViewMetrics>,
    /// Text-texture check, for objects with text textures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_check: Option<TextCheck>,
    /// Loader diagnostics.
    pub diagnostics: Diagnostics,
    /// Script-effect check (scripted objects, `--scripts`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_check: Option<ScriptCheck>,
    /// Audio block.
    pub audio: Audio,
    /// Timing.
    pub timing_ms: Timing,
    /// Command line that reproduces the case in the game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repro: Option<String>,
    /// Worker that produced the record.
    #[serde(default)]
    pub worker: usize,
}

impl Record {
    /// A record for an object the worker never finished (`F-CRASH`, `F-TIMEOUT`).
    #[must_use]
    pub fn aborted(entry: &ObjectEntry, category: &str, worker: usize, total_ms: u64) -> Self {
        Self {
            sco: entry.sco.clone(),
            key: entry.key.clone(),
            tier: entry.tier.clone(),
            verdict: Verdict::Fail,
            categories: vec![category.to_string()],
            audio: Audio {
                status: "recorded".into(),
                ..Audio::default()
            },
            timing_ms: Timing {
                total: total_ms,
                ..Timing::default()
            },
            worker,
            ..Self::default()
        }
    }
}
