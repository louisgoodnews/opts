//! The per-tile counts and the run status of the protocol.

use std::collections::BTreeMap;

use serde::Serialize;

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
