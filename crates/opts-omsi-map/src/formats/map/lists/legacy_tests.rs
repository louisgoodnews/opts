//! Unit tests of the legacy AI list syntax (`[ailist]`/`[aigroup]`), read through `AiLists`; the parser itself lives in `opts-omsi-ailist`.
#[cfg(test)]
mod tests {
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
