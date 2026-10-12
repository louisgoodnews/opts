//! Scenery object load verification (M-0054, `docs/src/architecture/scenery-benchmark.md`).
//!
//! A **supervisor** ([`supervisor`]) enumerates the `.sco` files of an OMSI installation and hands
//! them one at a time to **worker** processes ([`worker`]). A worker is a windowless Bevy app built
//! on `opts-game`'s map loader: it writes a one-tile map with the object ([`mapgen`]), lets the
//! loader rebuild the tile, renders the object off screen over black and white ([`capture`]),
//! rasterises the silhouette the object's own files predict ([`expected`], [`raster`]) and turns
//! the difference into metrics ([`metrics`]) and a verdict ([`verdict`]). Results go to
//! `results.jsonl` and the summary files ([`report`]).

pub mod capture;
pub mod cli;
pub mod corpus;
pub mod diag;
pub mod expected;
pub mod framing;
pub mod images;
pub mod lookups;
pub mod mapgen;
pub mod metrics;
pub mod object;
pub mod preflight;
pub mod protocol;
pub mod raster;
pub mod report;
pub mod script_check;
pub mod supervisor;
pub mod verdict;
pub mod worker;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Normalised relative path of a `.sco` file: forward slashes, as found on disk.
#[must_use]
pub fn normalise_rel(rel: &str) -> String {
    rel.replace('\\', "/")
}

/// The object key: the first 16 hex digits of the SHA-256 of the lower-case relative path
/// (forward slashes).
#[must_use]
pub fn object_key(rel: &str) -> String {
    let digest = Sha256::digest(normalise_rel(rel).to_lowercase().as_bytes());
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// Outcome of one object.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// No failure category.
    Pass,
    /// At least one failure category.
    Fail,
    /// No expected silhouette to compare with, or a borderline value.
    #[default]
    Unknown,
    /// Skipped on request.
    Skipped,
}

/// One object to verify, as listed in `objects.jsonl`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectEntry {
    /// Relative path below the OMSI root (forward slashes).
    pub sco: String,
    /// [`object_key`] of `sco`.
    pub key: String,
    /// `referenced` (placed by at least one map tile), `attachment` (only as a spline
    /// attachment) or `unreferenced`.
    pub tier: String,
}

/// Metrics of one view (`docs/src/architecture/scenery-benchmark.md`, Static check).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ViewMetrics {
    /// Rendered object pixels `|R|`.
    pub px_rendered: usize,
    /// Expected pixels `|E_all|`.
    pub px_expected: usize,
    /// Expected base pixels `|E_base|`.
    pub px_expected_base: usize,
    /// `|R ∩ dil(E_all, 3)| / |R|`.
    pub precision: Option<f32>,
    /// `|dil(R, 3) ∩ E_base| / |E_base|`.
    pub recall: Option<f32>,
    /// `|R ∩ E_all| / |R ∪ E_all|`.
    pub iou: Option<f32>,
    /// How far R's bounding box exceeds E_all's, as a share of E_all's diagonal.
    pub bbox_overshoot: Option<f32>,
    /// Distance of the mask centroids divided by E_all's diagonal.
    pub centroid_offset: Option<f32>,
    /// R's bounding-box extent divided by E_all's, per image axis.
    pub size_ratio: Option<[f32; 2]>,
    /// Share of `E_all` covered only by alpha-tested or blended materials.
    pub alpha_share: f32,
    /// Sentinel (magenta) pixels divided by `|R|`.
    pub sentinel_share: f32,
    /// V2 only: the alternative orientation that matches better than the identity, with its IoU.
    pub best_rotation: Option<(String, f32)>,
}

/// Feature tags of a record.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Tags {
    /// Keywords the object uses that OPTS does not implement for scenery.
    pub unsupported: Vec<String>,
    /// Script-effect status (`S-NOT-RUN` until a scenery-script runtime exists).
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
    /// Sound triggers raised by scripts (empty until Phase C).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_ignores_case_and_separators() {
        let a = object_key(r"Sceneryobjects\Animals\Cows.sco");
        let b = object_key("sceneryobjects/animals/cows.sco");
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
    }

    #[test]
    fn verdict_serialises_lower_case() {
        let s = serde_json::to_string(&Verdict::Pass).unwrap();
        assert_eq!(s, "\"pass\"");
    }
}
