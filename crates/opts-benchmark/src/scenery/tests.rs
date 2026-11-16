//! Unit tests for the scenery harness wiring.

use super::*;

#[test]
fn key_ignores_case_and_separators() {
    let a = object_key(r"Sceneryobjects\Animals\Cows.sco");
    let b = object_key("sceneryobjects/animals/cows.sco");
    assert_eq!(a, b);
    assert_eq!(a.len(), 16);
}

#[test]
fn verdict_serialises_lower_case() {
    let s = serde_json::to_string(&Verdict::Pass).unwrap();
    assert_eq!(s, "\"pass\"");
}
