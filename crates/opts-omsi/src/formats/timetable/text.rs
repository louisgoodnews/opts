//! Shared text helpers of the timetable parsers: the lines of a file, the keyword of a record and
//! its positional values.

use crate::formats::text::decode_bytes;

/// The lines of a file without carriage returns.
pub(super) fn lines_of(bytes: &[u8]) -> Vec<String> {
    decode_bytes(bytes)
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect()
}

/// The keyword of a line (`[addtrip]` gives `addtrip`), lower case; text after the closing
/// bracket (`[Postdrohne] Hildegardplatz`) is ignored.
pub(super) fn keyword(line: &str) -> Option<String> {
    let t = line.trim_start();
    let rest = t.strip_prefix('[')?;
    let end = rest.find(']')?;
    Some(rest[..end].trim().to_ascii_lowercase())
}

pub(super) fn at(lines: &[String], i: usize) -> &str {
    lines.get(i).map_or("", |s| s.trim())
}

pub(super) fn num<T: std::str::FromStr + Default>(s: &str) -> T {
    s.trim().replace(',', ".").parse().unwrap_or_default()
}
