//! The prepass gate: the cones wait for the prepass, then fade in.

use super::*;

#[test]
fn the_prepass_gate_waits_then_fades_in() {
    assert_eq!(prepass_gate(false, 0.0, 0), 1.0, "no fade, no gate");
    assert_eq!(prepass_gate(true, 0.0, 0), 0.0);
    assert_eq!(prepass_gate(true, PREPASS_READY_S - 0.1, 100), 0.0);
    assert_eq!(
        prepass_gate(true, PREPASS_READY_S + 5.0, 2),
        0.0,
        "frames too"
    );
    let mid = prepass_gate(true, PREPASS_READY_S + CONE_FADE_IN_S * 0.5, 100);
    assert!((mid - 0.5).abs() < 1e-5);
    assert_eq!(prepass_gate(true, PREPASS_READY_S + 10.0, 1000), 1.0);
}
