//! The plan describes itself for the protocol.

use super::*;

#[test]
fn the_plan_describes_itself_for_the_protocol() {
    let cat = catalogue();
    let r = record("MAN_SD77", "MAN SD77", "");
    let p = plan(
        &inputs(&cat, Some(&r)),
        "Grundorf",
        None,
        VehicleChoice::Carry,
    )
    .expect("plan");
    let d = p.describe();
    assert_eq!(d["map"], "Grundorf");
    assert_eq!(d["vehicle"], "MAN SD77");
    assert_eq!(d["time"], "08:45");
    assert_eq!(d["date"], "2026-03-14");
    assert_eq!(d["dropped"], serde_json::json!(["Citaro"]));
}

