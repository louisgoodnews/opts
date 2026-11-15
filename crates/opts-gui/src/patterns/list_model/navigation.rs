//! Keyboard stepping over the item rows and type-ahead to an entry by its title.

use super::{ListEntry, Row};

/// The first item row.
pub fn first_item(rows: &[Row]) -> Option<usize> {
    rows.iter().position(Row::is_item)
}

/// The last item row.
pub fn last_item(rows: &[Row]) -> Option<usize> {
    rows.iter().rposition(Row::is_item)
}

/// The item row `delta` items away from `from` (headers are skipped); stays at the ends.
pub fn step_item(rows: &[Row], from: Option<usize>, delta: isize) -> Option<usize> {
    let items: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.is_item())
        .map(|(i, _)| i)
        .collect();
    if items.is_empty() {
        return None;
    }
    let here = from.and_then(|f| items.iter().position(|i| *i >= f));
    let target = match here {
        Some(p) => p as isize + delta,
        None if delta < 0 => items.len() as isize - 1,
        None => 0,
    };
    Some(items[target.clamp(0, items.len() as isize - 1) as usize])
}

/// The item row that starts with `typed` (case-insensitive). A single letter looks *after* the
/// current row (so repeating a letter cycles), longer prefixes may stay on it; the search wraps.
pub fn type_ahead(
    entries: &[ListEntry],
    rows: &[Row],
    from: Option<usize>,
    typed: &str,
) -> Option<usize> {
    let typed = typed.to_lowercase();
    if typed.is_empty() {
        return None;
    }
    let n = rows.len();
    let start = match from {
        Some(f) if typed.chars().count() == 1 => f + 1,
        Some(f) => f,
        None => 0,
    };
    (0..n).map(|k| (start + k) % n).find(|i| {
        rows[*i]
            .entry()
            .is_some_and(|e| entries[e].title.to_lowercase().starts_with(&typed))
    })
}
