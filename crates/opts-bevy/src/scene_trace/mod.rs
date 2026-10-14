//! Scene trace (M-0087): which mesh is placed where.
//!
//! An opt-in debug mode that keeps one [`TraceRecord`] per placed thing (scenery object, tree,
//! parked vehicle, spline, terrain tile, traffic car, pedestrian, session vehicle) and writes
//! them as JSON Lines next to the run logs. The records answer the question *which source file is
//! this defect?* for people and for agents (control protocol `trace.*`).
//!
//! The mode is `off` (default), `summary` (per-tile counts and issues only) or `full` (every
//! record). **Off is free:** the hooks test one atomic ([`enabled`]); no record, string or
//! collection is built, and the state below is not even allocated (test `off_allocates_nothing`).
//!
//! See `docs/src/architecture/scene-trace.md`.

pub mod filter;
pub mod pick;
mod store;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use serde::{Deserialize, Serialize};

pub use filter::{Filters, Radius};
pub use store::{Status, TileSummary};

/// How much the trace keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Nothing is kept (default).
    #[default]
    Off,
    /// Counts per tile and kind, and a record for every thing with an issue.
    Summary,
    /// A record for every placed thing.
    Full,
}

impl Mode {
    /// Parses `off`, `summary` or `full` (case-insensitive); the empty text is `off`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim().to_ascii_lowercase().as_str() {
            "" | "off" | "false" | "0" | "no" => Some(Self::Off),
            "summary" | "sum" | "true" | "1" | "on" => Some(Self::Summary),
            "full" | "all" | "2" => Some(Self::Full),
            _ => None,
        }
    }

    /// The text form (`off`, `summary`, `full`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Summary => "summary",
            Self::Full => "full",
        }
    }

    const fn code(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Summary => 1,
            Self::Full => 2,
        }
    }

    const fn from_code(c: u8) -> Self {
        match c {
            1 => Self::Summary,
            2 => Self::Full,
            _ => Self::Off,
        }
    }
}

static MODE: AtomicU8 = AtomicU8::new(0);
static STATE: Mutex<Option<store::Store>> = Mutex::new(None);

/// The current mode.
#[must_use]
pub fn mode() -> Mode {
    Mode::from_code(MODE.load(Ordering::Relaxed))
}

/// Whether the trace is on. The one check every hook makes first.
#[inline]
#[must_use]
pub fn enabled() -> bool {
    MODE.load(Ordering::Relaxed) != 0
}

/// Whether the trace state is allocated (false while off; used by the zero-cost test).
#[must_use]
pub fn state_allocated() -> bool {
    STATE.lock().map(|s| s.is_some()).unwrap_or(false)
}

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
    /// Time in the decoder or its cache (ms).
    #[serde(default)]
    pub decode_ms: f32,
    /// Time building meshes, materials and textures (ms).
    #[serde(default)]
    pub build_ms: f32,
    /// Decoded vertices by texture-coordinate quality: exact, recovered, unreliable.
    #[serde(default)]
    pub uv: [usize; 3],
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
        matches!(
            self.stage.as_str(),
            "rejected" | "extent" | "read-failed" | "parse-failed"
        ) || !self.warnings.is_empty()
            || self.uv[2] * 10 > self.uv.iter().sum::<usize>().max(1)
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

    /// A short worst-quality label for colour coding: `failed`, `unreliable`, `decoded`, `ok`.
    #[must_use]
    pub fn quality(&self) -> &'static str {
        if self.result == "failed" || self.meshes.iter().any(|m| m.is_issue_stage()) {
            "failed"
        } else if self.meshes.iter().any(|m| m.uv[2] > 0) {
            "unreliable"
        } else if self.meshes.iter().any(|m| m.stage.starts_with("decode")) {
            "decoded"
        } else {
            "ok"
        }
    }
}

impl MeshNote {
    fn is_issue_stage(&self) -> bool {
        matches!(
            self.stage.as_str(),
            "rejected" | "extent" | "read-failed" | "parse-failed"
        )
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

/// Sets the directory the JSONL files go to (the game passes `<log directory>/trace`).
pub fn set_directory(dir: PathBuf) {
    store::set_directory(dir);
}

/// Sets the mode, allocating or dropping the state. Returns the previous mode.
pub fn set_mode(mode: Mode) -> Mode {
    let previous = self::mode();
    if mode == previous {
        return previous;
    }
    if let Ok(mut state) = STATE.lock() {
        if mode == Mode::Off {
            if let Some(s) = state.as_mut() {
                s.close("mode off");
            }
            *state = None;
        } else {
            let s = state.get_or_insert_with(store::Store::new);
            s.mode = mode;
            s.open_file_if_ready();
        }
    }
    MODE.store(mode.code(), Ordering::Relaxed);
    tracing::info!(mode = mode.name(), "scene trace mode");
    previous
}

/// Replaces the filters (the state must be on).
pub fn set_filters(filters: Filters) {
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.filters = filters;
        }
    }
}

/// The current filters.
#[must_use]
pub fn filters() -> Filters {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.filters.clone()))
        .unwrap_or_default()
}

/// Sets the size cap of the JSONL file in bytes (default 256 MiB).
pub fn set_size_cap(bytes: u64) {
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.size_cap = bytes;
        }
    }
}

/// A map session starts: names the run (the header record) and, when the trace is on, opens a
/// fresh file. `root` is the OMSI folder paths are made relative to.
pub fn begin_session(map: &str, root: &Path) {
    if !enabled() {
        // Remember for a later `set_mode`.
        store::remember_session(map, root);
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.begin_session(map, root);
        }
    }
}

/// A tile starts loading: earlier records of the same tile (a rebuild) are superseded.
pub fn begin_tile(key: (i32, i32)) {
    if !enabled() {
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.begin_tile(key);
        }
    }
}

/// A tile has been loaded: summary mode writes its counts line.
pub fn end_tile(key: (i32, i32)) {
    if !enabled() {
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.end_tile(key);
        }
    }
}

/// A tile was unloaded: its records leave the in-memory index (the file keeps them).
pub fn forget_tile(key: (i32, i32)) {
    if !enabled() {
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            s.forget_tile(key);
        }
    }
}

/// Whether a thing passes the filters (build the record only when it does).
#[must_use]
pub fn wants(probe: &Probe<'_>) -> bool {
    if !enabled() {
        return false;
    }
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.filters.accepts(probe)))
        .unwrap_or(false)
}

/// Records a thing. The closure runs only when the trace is on and the probe passes the filters,
/// so building the record costs nothing otherwise. Returns the record id.
pub fn record(probe: &Probe<'_>, build: impl FnOnce() -> TraceRecord) -> Option<u64> {
    if !wants(probe) {
        return None;
    }
    let mut rec = build();
    let mut state = STATE.lock().ok()?;
    let s = state.as_mut()?;
    Some(s.add(&mut rec))
}

/// Stores what the loader learned about a mesh file; later records pick it up with [`note_for`].
pub fn note_mesh(path: &Path, mut note: MeshNote) {
    if !enabled() {
        return;
    }
    if let Ok(mut state) = STATE.lock() {
        if let Some(s) = state.as_mut() {
            note.file = s.relative(path);
            s.notes.insert(store::norm(path), note);
        }
    }
}

/// Whether a note is stored for the file.
#[must_use]
pub fn has_note(path: &Path) -> bool {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.notes.contains_key(&store::norm(path))))
        .unwrap_or(false)
}

/// The note stored for a mesh file, or a bare `plain` note when the file was served from the
/// loader's caches without a fresh load.
#[must_use]
pub fn note_for(path: &Path) -> MeshNote {
    let Ok(state) = STATE.lock() else {
        return MeshNote::default();
    };
    let Some(s) = state.as_ref() else {
        return MeshNote::default();
    };
    s.notes
        .get(&store::norm(path))
        .cloned()
        .unwrap_or_else(|| MeshNote {
            file: s.relative(path),
            stage: "unknown".into(),
            ..MeshNote::default()
        })
}

/// A path relative to the OMSI folder (privacy: never an absolute path).
#[must_use]
pub fn relative(path: &Path) -> String {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.relative(path)))
        .unwrap_or_else(|| store::file_name_of(path))
}

/// Run status for `trace.set`.
#[must_use]
pub fn status() -> Status {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(store::Store::status))
        .unwrap_or_default()
}

/// Records matching a query, newest tile order preserved, at most `limit`.
#[must_use]
pub fn query(q: &Query) -> Vec<TraceRecord> {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|s| s.query(q)))
        .unwrap_or_default()
}

/// The record with `id`.
#[must_use]
pub fn get(id: u64) -> Option<TraceRecord> {
    STATE
        .lock()
        .ok()?
        .as_ref()?
        .records
        .iter()
        .find(|r| r.id == id)
        .cloned()
}

/// Per-tile counts.
#[must_use]
pub fn tile_summary(key: (i32, i32)) -> Option<TileSummary> {
    STATE.lock().ok()?.as_ref()?.summary_of(key)
}

/// Summaries of all tiles seen.
#[must_use]
pub fn summaries() -> Vec<TileSummary> {
    STATE
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(store::Store::summaries))
        .unwrap_or_default()
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

/// Every in-memory record that matches, as JSON Lines (for `trace.export`).
#[must_use]
pub fn export_lines(q: &Query) -> Vec<String> {
    query(q)
        .iter()
        .filter_map(|r| serde_json::to_string(r).ok())
        .collect()
}

/// A snapshot of records for the overlay (id, kind, quality, transform, bounds); bounded.
#[must_use]
pub fn overlay_snapshot(center: [f32; 2], radius: f32, limit: usize) -> Vec<TraceRecord> {
    query(&Query {
        near: Some((center, radius)),
        limit,
        ..Query::default()
    })
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

#[cfg(test)]
mod tests;
