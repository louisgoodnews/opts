//! The legacy AI list syntax (`docs/src/formats/map/lists.md`, "Legacy syntax"): an `[ailist]`
//! block holds one vehicle table, and each `[aigroup]` block names rows of that table by index.
//! Only the new-map template writes it; current maps use `[aigroup_2]`.

use super::{AiEntry, AiGroup};

/// The vehicle table of an `[ailist]` block: `0`, `-1`, the count, then that many file lines. Every
/// line of the table is kept, so the indices of `[aigroup]` keep their meaning.
pub(super) fn vehicle_table(parameters: &[String]) -> Vec<String> {
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
pub(super) fn group(parameters: &[String], table: &[String]) -> Option<AiGroup> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::map::lists::AiLists;

    #[test]
    fn legacy_group_names_rows_of_the_table_by_index() {
        // Table: A.bus, B.bus, C.bus. Indices 2 then 0 give C.bus then A.bus; "Spandau" is ignored.
        let text = b"[ailist]\n0\n-1\n3\nvehicles\\A.bus\nvehicles\\B.bus\nvehicles\\C.bus\n[aigroup]\nBusses\n2\n2\n0\nSpandau\n";
        let lists = AiLists::parse(text);
        assert_eq!(lists.groups.len(), 1);
        let g = lists.group("busses").expect("the legacy group");
        let files: Vec<&str> = g.entries.iter().map(|e| e.vehicle.as_str()).collect();
        assert_eq!(files, ["vehicles\\C.bus", "vehicles\\A.bus"]);
        assert_eq!(g.entries[0].weight, None);
        assert_eq!(g.total_weight(), 2.0);
    }

    #[test]
    fn out_of_range_indices_and_non_vehicle_rows_are_skipped() {
        // Table: A.bus, notes.txt. Index 0 is A.bus, 1 is not a vehicle, 9 is out of the table.
        let text = b"[ailist]\n0\n-1\n2\nvehicles\\A.bus\nnotes.txt\n[aigroup]\nMix\n3\n0\n1\n9\nSpandau\n";
        let lists = AiLists::parse(text);
        let g = lists.group("mix").expect("the legacy group");
        assert_eq!(g.entries.len(), 1);
        assert_eq!(g.entries[0].vehicle, "vehicles\\A.bus");
    }

    #[test]
    fn a_group_whose_count_is_not_a_number_is_not_read() {
        let lists = AiLists::parse(b"[aigroup]\nBad\nx\n0\n");
        assert!(lists.groups.is_empty());
    }
}
