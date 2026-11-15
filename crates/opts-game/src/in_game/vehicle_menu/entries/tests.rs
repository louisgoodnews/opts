//! Unit tests for `super` (`entries.rs`).

use super::lists::numbered;

use super::*;

#[test]
fn stops_that_share_a_name_are_numbered() {
    let rows = vec![
        (7, "Rathaus".to_string()),
        (8, "Rathaus".to_string()),
        (3, "Markt".to_string()),
    ];
    assert_eq!(
        numbered(rows),
        vec![
            (7, "Rathaus (1)".to_string()),
            (8, "Rathaus (2)".to_string()),
            (3, "Markt".to_string()),
        ]
    );
}
