//! Trace records: one placed thing, the things it belongs to, and whether it is an issue.

use serde::{Deserialize, Serialize};

use super::geometry::{TraceBounds, TraceTransform};
use super::mesh::MeshNote;

/// Reference to a thing a record belongs to (a spline carrying an attachment, the marker that
/// a parked car replaced, the vehicle a part belongs to).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ParentRef {
    /// What the parent is (a `kind` or `marker`).
    pub kind: String,
    /// File of the parent.
    #[serde(default)]
    pub file: String,
    /// Tile of the parent.
    #[serde(default)]
    pub tile: Option<[i32; 2]>,
    /// Record index of the parent in its tile.
    #[serde(default)]
    pub index: Option<usize>,
    /// Trace id of the parent record, when it has one.
    #[serde(default)]
    pub id: Option<u64>,
}

/// One placed thing.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TraceRecord {
    /// Running number within the run.
    #[serde(default)]
    pub id: u64,
    /// Milliseconds since the trace started.
    #[serde(default)]
    pub t_ms: u64,
    /// One of `kind`.
    pub kind: String,
    /// Tile `(x, y)`.
    #[serde(default)]
    pub tile: Option<[i32; 2]>,
    /// Tile file, relative to the OMSI folder.
    #[serde(default)]
    pub tile_file: Option<String>,
    /// Record index in the tile (`objects`, `splines` or the attachment list, see `list`).
    #[serde(default)]
    pub index: Option<usize>,
    /// Which list of the tile `index` counts in: `objects`, `splines`, `attachments`.
    #[serde(default)]
    pub list: Option<String>,
    /// Source file of the thing (`.sco`, `.sli`, `.bus`, `.hum`, `.ai` ...), relative to OMSI.
    #[serde(default)]
    pub file: String,
    /// Position in the tile's OMSI coordinates (x, y, z).
    #[serde(default)]
    pub pos: Option<[f32; 3]>,
    /// Heading, pitch and roll in degrees as stored in the map.
    #[serde(default)]
    pub rot_deg: Option<[f32; 3]>,
    /// World transform in Bevy space.
    #[serde(default)]
    pub world: Option<TraceTransform>,
    /// Local bounding box of the geometry.
    #[serde(default)]
    pub bounds: Option<TraceBounds>,
    /// Bevy entity (bits) of the thing's root.
    #[serde(default)]
    pub entity: Option<u64>,
    /// The mesh files used, with decode stage, quality, materials and timings.
    #[serde(default)]
    pub meshes: Vec<MeshNote>,
    /// Things this one belongs to.
    #[serde(default)]
    pub parents: Vec<ParentRef>,
    /// `ok`, `no-geometry` (nothing to draw by design) or `failed`.
    #[serde(default)]
    pub result: String,
    /// Problems seen while loading.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Time spent loading the thing (ms).
    #[serde(default)]
    pub load_ms: f32,
    /// Free-form details (spline profile, vehicle name, livery ...).
    #[serde(default)]
    pub detail: Option<serde_json::Value>,
}

impl TraceRecord {
    /// Whether the record describes a problem (summary mode keeps only these).
    #[must_use]
    pub fn is_issue(&self) -> bool {
        self.result == "failed"
            || !self.warnings.is_empty()
            || self.meshes.iter().any(MeshNote::is_issue)
    }

    /// A short worst-quality label for colour coding: `failed`, `decoded`, `ok`.
    #[must_use]
    pub fn quality(&self) -> &'static str {
        if self.result == "failed" || self.meshes.iter().any(|m| m.is_issue_stage()) {
            "failed"
        } else if self.meshes.iter().any(|m| m.stage == "exact") {
            "decoded"
        } else {
            "ok"
        }
    }
}
