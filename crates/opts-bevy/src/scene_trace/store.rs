//! State of a running trace: in-memory index, per-tile counts and the JSONL writer.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use serde_json::json;

use super::{filter::Filters, Mode, Query, TraceRecord};

mod query;
use query::prune;
mod index;
mod paths;
mod status;
mod writer;

pub(super) use paths::*;
pub use status::*;


/// Records kept in memory; more are written to the file only.
pub const MAX_MEMORY_RECORDS: usize = 150_000;
/// Default size cap of one JSONL file.
pub const DEFAULT_SIZE_CAP: u64 = 256 * 1024 * 1024;
/// Trace files kept in the directory (the oldest are removed beyond it).
pub const KEEP_FILES: usize = 10;

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
}

