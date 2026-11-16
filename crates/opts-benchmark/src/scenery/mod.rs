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
pub mod script_judge;
pub mod script_run;
pub mod script_stimuli;
pub mod supervisor;
pub mod verdict;
pub mod worker;

mod record;
pub use record::{Audio, Diagnostics, EventRecord, Record, Tags, TextCheck, Timing};

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

#[cfg(test)]
mod tests;
