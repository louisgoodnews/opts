//! `objects.jsonl`: writing and reading the corpus entries.

use std::io::{BufRead, Write as _};
use std::path::Path;

use anyhow::Result;

use super::super::ObjectEntry;

/// Writes `objects.jsonl`.
///
/// # Errors
/// I/O failures.
pub fn write_jsonl(path: &Path, entries: &[ObjectEntry]) -> Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    for e in entries {
        writeln!(f, "{}", serde_json::to_string(e)?)?;
    }
    f.flush()?;
    Ok(())
}

/// Reads `objects.jsonl`.
///
/// # Errors
/// I/O or JSON failures.
pub fn read_jsonl(path: &Path) -> Result<Vec<ObjectEntry>> {
    let f = std::io::BufReader::new(std::fs::File::open(path)?);
    f.lines().map(|l| Ok(serde_json::from_str(&l?)?)).collect()
}
