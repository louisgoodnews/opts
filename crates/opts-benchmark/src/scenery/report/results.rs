//! `results.jsonl`: the appended record file, its reader and the latest record per object.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::Result;

use super::super::Record;

/// Appends records to `results.jsonl`, flushing after each one.
#[derive(Debug)]
pub struct ResultsWriter(std::fs::File);

impl ResultsWriter {
    /// Opens (or creates) `results.jsonl` for appending.
    ///
    /// # Errors
    /// I/O failures.
    pub fn open(path: &Path) -> Result<Self> {
        let f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self(f))
    }

    /// Appends one record.
    ///
    /// # Errors
    /// I/O or JSON failures.
    pub fn append(&mut self, r: &Record) -> Result<()> {
        writeln!(self.0, "{}", serde_json::to_string(r)?)?;
        self.0.flush()?;
        Ok(())
    }
}

/// Reads `results.jsonl`; a truncated last line (crash while writing) is skipped.
#[must_use]
pub fn read_results(path: &Path) -> Vec<Record> {
    let Ok(f) = std::fs::File::open(path) else {
        return Vec::new();
    };
    std::io::BufReader::new(f)
        .lines()
        .map_while(Result::ok)
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect()
}

/// The last record per key (a resumed run may have retried an object).
#[must_use]
pub fn latest(records: Vec<Record>) -> Vec<Record> {
    let mut by_key: BTreeMap<String, Record> = BTreeMap::new();
    for r in records {
        by_key.insert(r.key.clone(), r);
    }
    by_key.into_values().collect()
}
