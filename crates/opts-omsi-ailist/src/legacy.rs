//! The legacy AI list syntax (`docs/src/formats/map/lists.md`, "Legacy syntax"): an `[ailist]`
//! block holds one vehicle table, and each `[aigroup]` block names rows of that table by index.
//! Only the new-map template writes it; current maps use `[aigroup_2]`.

use super::{AiEntry, AiGroup};

/// The vehicle table of an `[ailist]` block: `0`, `-1`, the count, then that many file lines. Every
/// line of the table is kept, so the indices of `[aigroup]` keep their meaning.
pub fn vehicle_table(parameters: &[String]) -> Vec<String> {
    let count = parameters
        .get(2)
        .and_then(|c| c.trim().parse::<usize>().ok())
        .unwrap_or(0);
    parameters
        .iter()
        .skip(3)
        .take(count)
        .map(|f| f.trim().to_owned())
        .collect()
}

/// The group of an `[aigroup]` block: its name, a count, that many indices into `table`, then a
/// trailing name that is not used here (`Spandau` in the template, meaning unknown). Weights do not
/// exist in this syntax, so every entry has none. `None` when the count is not a number.
pub fn group(parameters: &[String], table: &[String]) -> Option<AiGroup> {
    let name = parameters.first()?.trim().to_owned();
    let count = parameters.get(1)?.trim().parse::<usize>().ok()?;
    let entries = parameters
        .iter()
        .skip(2)
        .take(count)
        .filter_map(|i| table.get(i.trim().parse::<usize>().ok()?))
        .filter(|file| is_vehicle(file))
        .map(|file| AiEntry {
            vehicle: file.clone(),
            weight: None,
        })
        .collect();
    Some(AiGroup { name, entries })
}

/// Whether a table line names a vehicle file (`.bus`, `.ovh` or `.zug`), as the `_2` groups require.
fn is_vehicle(file: &str) -> bool {
    let lower = file.to_ascii_lowercase();
    [".bus", ".ovh", ".zug"].iter().any(|e| lower.ends_with(e))
}
