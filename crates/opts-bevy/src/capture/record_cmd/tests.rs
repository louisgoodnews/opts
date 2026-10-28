//! Unit tests for `super` (`record_cmd.rs`).

use super::*;

#[test]
fn parameters_are_validated_against_the_limits() {
    let l = CaptureLimits::default();
    let ok = parse(&json!({}), &l).expect("defaults");
    assert_eq!(
        (ok.fps, ok.duration_ms, ok.format),
        (30, 60_000, RecFormat::Av1)
    );
    for bad in [
        json!({"fps": 0}),
        json!({"fps": 61}),
        json!({"duration_ms": 0}),
        json!({"duration_ms": 1_800_001}),
        json!({"max_size_mb": 2049}),
        json!({"scale": 0.1}),
        json!({"format": "gif"}),
        json!({"format": "h264"}),
        json!({"quality": 101}),
    ] {
        assert_eq!(
            parse(&bad, &l).err().map(|e| e.code),
            Some(code::INVALID_PARAMS),
            "{bad}"
        );
    }
    assert!(parse(&json!({"duration_ms": 1_800_000, "max_size_mb": 2048}), &l).is_ok());
}
