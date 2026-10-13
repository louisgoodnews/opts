//! Time budget, schema round trip and unknown-field handling.

use std::time::{Duration, Instant};

use opts_hardware::inputs::ProbeInputs;
use opts_hardware::{probe, probe_with, Fact, HardwareProfile, Reason, SCHEMA_VERSION};

/// The real probe finishes within the budget plus the time of one stage that was already running
/// when the budget ended. The reference machine needs far less than 500 ms; CI runners get slack
/// (the probe itself stops starting new stages after 500 ms).
#[test]
fn full_probe_stays_within_the_time_budget() {
    let start = Instant::now();
    let p = probe();
    let took = start.elapsed();
    assert!(took < Duration::from_millis(2000), "probe took {took:?}");
    assert!(u128::from(p.probe_ms) <= took.as_millis() + 1);
}

#[test]
fn exhausted_budget_skips_stages_with_a_timeout_reason() {
    let p = probe_with(&ProbeInputs {
        budget: Duration::ZERO,
        ..ProbeInputs::default()
    });
    assert_eq!(p.cpu.logical_cores, Fact::Unknown(Reason::Timeout));
    assert_eq!(p.memory.total_mib, Fact::Unknown(Reason::Timeout));
    assert_eq!(p.storage.free_gib, Fact::Unknown(Reason::Timeout));
    assert_eq!(p.gpu.vendor, Fact::Unknown(Reason::Timeout));
}

#[test]
fn real_probe_knows_the_basics_on_this_machine() {
    let p = probe();
    assert_eq!(p.schema, SCHEMA_VERSION);
    assert!(p.cpu.logical_cores.get().is_some_and(|c| c >= 1));
    assert!(p.memory.total_mib.get().is_some_and(|m| m > 0));
    assert!(p.os.family.known().is_some());
    // Everything the renderer owns is explicit about why it is unknown.
    if p.gpu.max_texture_2d.known().is_none() {
        assert_eq!(p.gpu.max_texture_2d.reason(), Some(Reason::NeedsRenderer));
    }
}

#[test]
fn schema_round_trip() {
    let p = probe();
    let back = HardwareProfile::from_json(&p.to_json()).unwrap();
    assert_eq!(back, p);
}

#[test]
fn unknown_fields_are_ignored_and_missing_ones_are_unprobed() {
    let text = r#"{"schema":1,"future_section":{"x":1},"cpu":{"logical_cores":{"known":8},"new_field":{"known":1}}}"#;
    let p = HardwareProfile::from_json(text).unwrap();
    assert_eq!(p.cpu.logical_cores, Fact::Known(8));
    assert_eq!(p.cpu.physical_cores, Fact::Unknown(Reason::NotProbed));
    assert_eq!(p.memory.total_mib, Fact::Unknown(Reason::NotProbed));
}

#[test]
fn newer_schema_and_garbage_are_refused() {
    assert!(HardwareProfile::from_json(r#"{"schema":2}"#).is_err());
    assert!(HardwareProfile::from_json("nonsense").is_err());
    assert!(HardwareProfile::from_json(r#"{"cpu":{"arch":{"known":"sparc"}}}"#).is_err());
}

#[test]
fn class_hash_is_stable_and_coarse() {
    let mut a = HardwareProfile::default();
    a.memory.total_mib = Fact::Known(16_000);
    let mut b = a.clone();
    b.memory.total_mib = Fact::Known(16_300);
    b.memory.available_mib = Fact::Known(3);
    assert_eq!(
        a.class_hash(),
        b.class_hash(),
        "same RAM bucket, same class"
    );
    b.memory.total_mib = Fact::Known(64_000);
    assert_ne!(a.class_hash(), b.class_hash());
    assert_eq!(a.class_hash().len(), 16);
}
