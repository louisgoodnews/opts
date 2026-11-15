//! Unit tests for `super` (`ui_id.rs`).

use super::*;

#[test]
fn slugs_are_short_lower_case_and_dash_separated() {
    assert_eq!(slug("Load Last Game"), "load-last-game");
    assert_eq!(slug("  Ärger & Co.  "), "ärger-co");
    assert_eq!(slug(""), "");
    assert!(slug(&"x".repeat(80)).chars().count() <= 32);
}
