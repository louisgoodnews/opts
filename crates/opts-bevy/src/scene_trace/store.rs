//! State of a running trace: in-memory index, per-tile counts and the JSONL writer.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::json;

use super::{filter::Filters, Mode, Query, TraceRecord};

/// Records kept in memory; more are written to the file only.
pub const MAX_MEMORY_RECORDS: usize = 150_000;
/// Default size cap of one JSONL file.
pub const DEFAULT_SIZE_CAP: u64 = 256 * 1024 * 1024;
/// Trace files kept in the directory (the oldest are removed beyond it).
pub const KEEP_FILES: usize = 10;

static DIRECTORY: Mutex<Option<PathBuf>> = Mutex::new(None);
static SESSION: Mutex<Option<(String, PathBuf)>> = Mutex::new(None);

pub(super) fn set_directory(dir: PathBuf) {
    if let Ok(mut d) = DIRECTORY.lock() {
        *d = Some(dir);
    }
}

fn directory() -> PathBuf {
    DIRECTORY
        .lock()
        .ok()
        .and_then(|d| d.clone())
        .unwrap_or_else(|| std::env::temp_dir().join("opts-trace"))
}

pub(super) fn remember_session(map: &str, root: &Path) {
    if let Ok(mut s) = SESSION.lock() {
        *s = Some((map.to_owned(), root.to_path_buf()));
    }
}

pub(super) fn norm(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/").to_lowercase()
}

pub(super) fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Counts of one tile.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct TileSummary {
    /// Tile `(x, y)`.
    pub tile: [i32; 2],
    /// Records per kind.
    pub kinds: BTreeMap<String, usize>,
    /// Records with issues per kind.
    pub issues: BTreeMap<String, usize>,
    /// Decode stage of mesh files used on the tile, counted per record.
    pub stages: BTreeMap<String, usize>,
    /// Total records.
    pub total: usize,
    /// Total records with issues.
    pub issue_total: usize,
}

/// Run status for the protocol.
#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub struct Status {
    /// `off`, `summary`, `full`.
    pub mode: String,
    /// JSONL file of the run, if one is open.
    pub file: Option<String>,
    /// Bytes written.
    pub bytes: u64,
    /// Whether the size cap stopped the file.
    pub truncated: bool,
    /// Records seen (including filtered-in ones not kept in memory).
    pub records: u64,
    /// Records in the in-memory index.
    pub in_memory: usize,
    /// Records with issues.
    pub issues: u64,
    /// Map of the run.
    pub map: Option<String>,
    /// Tiles with records.
    pub tiles: usize,
}

pub(super) struct Store {
    pub mode: Mode,
    pub filters: Filters,
    pub size_cap: u64,
    pub records: Vec<TraceRecord>,
    pub notes: HashMap<String, super::MeshNote>,
    tiles: BTreeMap<(i32, i32), TileSummary>,
    next_id: u64,
    seen: u64,
    issues: u64,
    started: Instant,
    root: Option<PathBuf>,
    map: Option<String>,
    writer: Option<BufWriter<File>>,
    file: Option<PathBuf>,
    bytes: u64,
    truncated: bool,
}

impl Store {
    pub fn new() -> Self {
        let (map, root) = SESSION
            .lock()
            .ok()
            .and_then(|s| s.clone())
            .map_or((None, None), |(m, r)| (Some(m), Some(r)));
        Self {
            mode: Mode::Summary,
            filters: Filters::default(),
            size_cap: DEFAULT_SIZE_CAP,
            records: Vec::new(),
            notes: HashMap::new(),
            tiles: BTreeMap::new(),
            next_id: 1,
            seen: 0,
            issues: 0,
            started: Instant::now(),
            root,
            map,
            writer: None,
            file: None,
            bytes: 0,
            truncated: false,
        }
    }

    pub fn relative(&self, path: &Path) -> String {
        let text = path.to_string_lossy().replace('\\', "/");
        if let Some(root) = &self.root {
            let root = root.to_string_lossy().replace('\\', "/");
            let root = root.trim_end_matches('/');
            if text.len() > root.len()
                && text[..root.len()].eq_ignore_ascii_case(root)
                && text.as_bytes()[root.len()] == b'/'
            {
                return text[root.len() + 1..].to_owned();
            }
        }
        if path.is_absolute() {
            // Outside the installation (a test fixture, a user folder): the name only.
            return file_name_of(path);
        }
        text
    }

    pub fn begin_session(&mut self, map: &str, root: &Path) {
        self.map = Some(map.to_owned());
        self.root = Some(root.to_path_buf());
        self.records.clear();
        self.tiles.clear();
        self.notes.clear();
        self.seen = 0;
        self.issues = 0;
        self.next_id = 1;
        self.started = Instant::now();
        self.close("new session");
        self.open_file_if_ready();
    }

    pub fn open_file_if_ready(&mut self) {
        if self.writer.is_some() || self.map.is_none() {
            return;
        }
        let dir = directory();
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        let map = self.map.clone().unwrap_or_default();
        let safe: String = map
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let path = dir.join(format!("scene-trace-{stamp}-{safe}.jsonl"));
        let Ok(file) = File::create(&path) else {
            tracing::warn!(path = %path.display(), "scene trace file could not be created");
            return;
        };
        self.writer = Some(BufWriter::new(file));
        self.file = Some(path.clone());
        self.bytes = 0;
        self.truncated = false;
        let header = json!({
            "type": "header",
            "schema": 1,
            "build": super::build_info(),
            "map": map,
            "mode": self.mode.name(),
            "started_unix": stamp,
            "filters": {
                "tiles": self.filters.tiles,
                "kinds": self.filters.kinds,
                "file": self.filters.file,
            },
            "size_cap": self.size_cap,
        });
        self.write_value(&header);
        tracing::info!(path = %path.display(), mode = self.mode.name(), "scene trace file opened");
        prune(&dir);
    }

    pub fn close(&mut self, why: &str) {
        if self.writer.is_some() {
            let footer = json!({
                "type": "footer", "why": why, "records": self.seen,
                "issues": self.issues, "truncated": self.truncated,
            });
            self.write_value_raw(&footer);
            if let Some(w) = self.writer.as_mut() {
                let _ = w.flush();
            }
            tracing::info!(
                records = self.seen,
                issues = self.issues,
                file = ?self.file,
                "scene trace closed: {why}"
            );
        }
        self.writer = None;
    }

    fn write_value(&mut self, v: &impl Serialize) {
        if self.truncated {
            return;
        }
        if self.bytes >= self.size_cap {
            self.truncated = true;
            self.write_value_raw(&json!({"type": "truncated", "cap": self.size_cap}));
            tracing::warn!(cap = self.size_cap, "scene trace file reached its size cap");
            return;
        }
        self.write_value_raw(v);
    }

    fn write_value_raw(&mut self, v: &impl Serialize) {
        let Some(w) = self.writer.as_mut() else {
            return;
        };
        if let Ok(line) = serde_json::to_string(v) {
            if writeln!(w, "{line}").is_ok() {
                self.bytes += line.len() as u64 + 1;
            }
        }
    }

    pub fn begin_tile(&mut self, key: (i32, i32)) {
        let had = self.tiles.remove(&key).is_some();
        self.records.retain(|r| r.tile != Some([key.0, key.1]));
        if had {
            self.write_value(&json!({"type": "tile_reload", "tile": [key.0, key.1]}));
        }
    }

    pub fn end_tile(&mut self, key: (i32, i32)) {
        if let Some(s) = self.tiles.get(&key).cloned() {
            self.write_value(&json!({"type": "tile_summary", "summary": s}));
        }
    }

    pub fn forget_tile(&mut self, key: (i32, i32)) {
        self.records.retain(|r| r.tile != Some([key.0, key.1]));
        self.write_value(&json!({"type": "tile_unload", "tile": [key.0, key.1]}));
    }

    pub fn add(&mut self, rec: &mut TraceRecord) -> u64 {
        rec.id = self.next_id;
        self.next_id += 1;
        rec.t_ms = self.started.elapsed().as_millis() as u64;
        self.seen += 1;
        let issue = rec.is_issue();
        if rec.result.is_empty() {
            rec.result = if issue { "failed" } else { "ok" }.into();
        }
        if issue {
            self.issues += 1;
        }
        if let Some(t) = rec.tile {
            let s = self
                .tiles
                .entry((t[0], t[1]))
                .or_insert_with(|| TileSummary {
                    tile: t,
                    ..TileSummary::default()
                });
            s.total += 1;
            *s.kinds.entry(rec.kind.clone()).or_default() += 1;
            if issue {
                s.issue_total += 1;
                *s.issues.entry(rec.kind.clone()).or_default() += 1;
            }
            let mut stages: Vec<&str> = rec.meshes.iter().map(|m| m.stage.as_str()).collect();
            stages.sort_unstable();
            stages.dedup();
            for st in stages {
                *s.stages.entry(st.to_owned()).or_default() += 1;
            }
        }
        let keep = self.mode == Mode::Full || issue;
        if keep {
            let line = json!({"type": "record", "record": &*rec});
            self.write_value(&line);
            if self.records.len() < MAX_MEMORY_RECORDS {
                self.records.push(rec.clone());
            }
        }
        rec.id
    }

    pub fn status(&self) -> Status {
        Status {
            mode: self.mode.name().into(),
            file: self.file.as_ref().map(|p| p.display().to_string()),
            bytes: self.bytes,
            truncated: self.truncated,
            records: self.seen,
            in_memory: self.records.len(),
            issues: self.issues,
            map: self.map.clone(),
            tiles: self.tiles.len(),
        }
    }

    pub fn summary_of(&self, key: (i32, i32)) -> Option<TileSummary> {
        self.tiles.get(&key).cloned()
    }

    pub fn summaries(&self) -> Vec<TileSummary> {
        self.tiles.values().cloned().collect()
    }

    pub fn query(&self, q: &Query) -> Vec<TraceRecord> {
        let limit = if q.limit == 0 { 200 } else { q.limit };
        let mut out = Vec::new();
        for r in &self.records {
            if q.tile.is_some_and(|t| r.tile != Some([t.0, t.1])) {
                continue;
            }
            if q.kind.as_deref().is_some_and(|k| r.kind != k) {
                continue;
            }
            if q.issues_only && !r.is_issue() {
                continue;
            }
            if let Some(p) = &q.file {
                let hit = super::filter::glob_match(p, &r.file)
                    || r.meshes
                        .iter()
                        .any(|m| super::filter::glob_match(p, &m.file));
                if !hit {
                    continue;
                }
            }
            if let Some(st) = &q.stage {
                if !r.meshes.iter().any(|m| &m.stage == st) {
                    continue;
                }
            }
            if let Some((c, rad)) = q.near {
                let Some(w) = r.world else { continue };
                let (dx, dz) = (w.t[0] - c[0], w.t[2] - c[1]);
                if dx * dx + dz * dz > rad * rad {
                    continue;
                }
            }
            out.push(r.clone());
            if out.len() >= limit {
                break;
            }
        }
        out
    }
}

fn prune(dir: &Path) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(SystemTime, PathBuf)> = rd
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("scene-trace-"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    files.sort();
    while files.len() > KEEP_FILES {
        let (_, p) = files.remove(0);
        let _ = std::fs::remove_file(p);
    }
}
