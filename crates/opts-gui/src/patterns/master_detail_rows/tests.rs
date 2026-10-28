//! Unit tests for `super` (`master_detail_rows.rs`).

use super::ellipsize;

#[test]
fn long_titles_are_cut_with_an_ellipsis_and_short_ones_are_kept() {
    assert_eq!(ellipsize("Berlin", 20), "Berlin");
    assert_eq!(ellipsize("Aachener Stadtbus Variante", 10), "Aachener …");
    assert_eq!(ellipsize("äöü", 3), "äöü");
}
