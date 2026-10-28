//! Unit tests for `super` (`job.rs`).

use super::*;

#[test]
fn car_park_value_selects_the_parking_list() {
    let file = |v| parklist_file_name(parklist_selector(v));
    assert_eq!(file(None), "parklist_p.txt");
    assert_eq!(file(Some("")), "parklist_p.txt");
    assert_eq!(file(Some("0")), "parklist_p.txt");
    assert_eq!(file(Some("1")), "parklist_p_1.txt");
    assert_eq!(file(Some(" 2 ")), "parklist_p_2.txt");
    assert_eq!(file(Some("Trabant")), "parklist_p.txt");
}
