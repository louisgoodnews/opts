//! Unit tests for script_check.

use super::*;

#[test]
fn thresholds_round_trip_and_default_when_missing() {
    let t = ScriptThresholds::default();
    let json = serde_json::to_string(&t).unwrap_or_default();
    let back: ScriptThresholds = serde_json::from_str(&json).unwrap_or_default();
    assert_eq!(t, back);
    let partial: ScriptThresholds = serde_json::from_str("{}").unwrap_or_default();
    assert_eq!(partial, t);
}
