//! Data structures for scene trace records, mesh notes, probes, bounds, and transforms.

use super::*;

/// What kind of thing a record describes.
pub mod kind {
    /// A placed scenery object (`.sco`).
    pub const SCENERY: &str = "scenery";
    /// A tree of the map.
    pub const TREE: &str = "tree";
    /// An object placed along a spline (attachment).
    pub const ATTACHMENT: &str = "attachment";
    /// A parked or static vehicle placed as scenery.
    pub const PARKED: &str = "parked";
    /// A spline segment (road, rail, fence ...).
    pub const SPLINE: &str = "spline";
    /// The terrain mesh of a tile.
    pub const TERRAIN: &str = "terrain";
    /// An AI traffic vehicle.
    pub const TRAFFIC: &str = "traffic";
    /// A pedestrian.
    pub const HUMAN: &str = "human";
    /// A vehicle of the session (player, session vehicle).
    pub const VEHICLE: &str = "vehicle";
    /// All kinds, for validation.
    pub const ALL: [&str; 9] = [
        SCENERY, TREE, ATTACHMENT, PARKED, SPLINE, TERRAIN, TRAFFIC, HUMAN, VEHICLE,
    ];
}

/// A cheap, borrowed description of a thing, tested against the filters before a record is built.
#[derive(Debug, Clone, Copy)]
pub struct Probe<'a> {
    /// One of [`kind`].
    pub kind: &'a str,
    /// Tile the thing belongs to.
    pub tile: Option<(i32, i32)>,
    /// Source file (any form; matched case-insensitively).
    pub file: &'a str,
    /// Position in Bevy world space (x, z are used by the radius filter).
    pub world: [f32; 3],
}

/// How one mesh file was obtained and what it looks like; written once per file and run.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MeshNote {
    /// File, relative to the OMSI folder.
    #[serde(default)]
    pub file: String,
    /// How the file became geometry: `plain`, `twin`, `decode-cache`, `decode-background`,
    /// `decoded-now`, `decode-deferred`, `rejected` (scrambled and not decodable), `extent`
    /// (implausible size), `read-failed`, `parse-failed`, `hidden` (every slot variable-hidden).
    pub stage: String,
    /// Time reading the file (ms).
    #[serde(default)]
    pub read_ms: f32,
    /// Time parsing (ms).
    #[serde(default)]
    pub parse_ms: f32,
    /// Time decoding a keyed mesh (ms).
    #[serde(default)]
    pub decode_ms: f32,
    /// Time building meshes, materials and textures (ms).
    #[serde(default)]
    pub build_ms: f32,
    /// Vertices of the file.
    #[serde(default)]
    pub vertices: usize,
    /// Parts (one per material slot) produced.
    #[serde(default)]
    pub parts: usize,
    /// Material slots with texture, alpha mode and result.
    #[serde(default)]
    pub materials: Vec<MaterialNote>,
    /// Warnings of the load.
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Bounding box of the produced geometry (Bevy axes, the file's own space).
    #[serde(default)]
    pub bbox: Option<TraceBounds>,
}

/// One material slot of a mesh as it was built.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MaterialNote {
    /// Material slot of the `.o3d`.
    pub slot: usize,
    /// Texture reference as given (empty when none).
    #[serde(default)]
    pub texture: String,
    /// `opaque`, `mask` or `blend`.
    pub alpha: String,
    /// Whether a texture image was found and bound.
    pub textured: bool,
}

impl MeshNote {
    /// Whether the stage means the file produced no geometry or a doubtful one.
    #[must_use]
    pub fn is_issue(&self) -> bool {
        self.is_issue_stage() || !self.warnings.is_empty()
    }

    pub(super) fn is_issue_stage(&self) -> bool {
        matches!(
            self.stage.as_str(),
            "rejected" | "extent" | "read-failed" | "parse-failed"
        )
    }
}

/// A transform in Bevy space.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TraceTransform {
    /// Translation.
    pub t: [f32; 3],
    /// Rotation as quaternion (x, y, z, w).
    pub q: [f32; 4],
    /// Scale.
    pub s: [f32; 3],
}

/// Local-space bounding box of the thing's geometry.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct TraceBounds {
    /// Smallest corner.
    pub min: [f32; 3],
    /// Largest corner.
    pub max: [f32; 3],
}

/// Reference to a thing a record belongs to (a spline carrying an attachment, the marker that
/// a parked car replaced, the vehicle a part belongs to).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ParentRef {
    /// What the parent is (a [`kind`] or `marker`).
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
    /// One of [`kind`].
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

/// Union of the bounding boxes of the notes' geometry.
#[must_use]
pub fn union_bounds(notes: &[MeshNote]) -> Option<TraceBounds> {
    let mut out: Option<TraceBounds> = None;
    for b in notes.iter().filter_map(|n| n.bbox) {
        out = Some(match out {
            None => b,
            Some(o) => TraceBounds {
                min: std::array::from_fn(|i| o.min[i].min(b.min[i])),
                max: std::array::from_fn(|i| o.max[i].max(b.max[i])),
            },
        });
    }
    out
}

/// Build information written into the header record.
#[must_use]
pub fn build_info() -> serde_json::Value {
    serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
    })
}

/// A query over the in-memory records.
#[derive(Debug, Clone, Default)]
pub struct Query {
    /// Only this tile.
    pub tile: Option<(i32, i32)>,
    /// Only this kind.
    pub kind: Option<String>,
    /// File pattern (substring, `*` wildcards, case-insensitive) matched against the record's
    /// file and its mesh files.
    pub file: Option<String>,
    /// Only records with issues.
    pub issues_only: bool,
    /// Within `radius` of `center` (Bevy x, z).
    pub near: Option<([f32; 2], f32)>,
    /// Only records with this decode stage in one of their meshes.
    pub stage: Option<String>,
    /// Most records returned.
    pub limit: usize,
}

/// Measures a stage when the trace is on; a no-op `Lap` otherwise.
pub struct Lap(Option<Instant>);

impl Lap {
    /// Starts a lap if the trace is on.
    #[must_use]
    pub fn start() -> Self {
        Self(enabled().then(Instant::now))
    }

    /// Milliseconds since the last call (or start); 0 when off.
    pub fn split(&mut self) -> f32 {
        match &mut self.0 {
            Some(t) => {
                let now = Instant::now();
                let ms = now.duration_since(*t).as_secs_f32() * 1000.0;
                *t = now;
                ms
            }
            None => 0.0,
        }
    }

    /// Whether the lap measures.
    #[must_use]
    pub fn active(&self) -> bool {
        self.0.is_some()
    }
}
