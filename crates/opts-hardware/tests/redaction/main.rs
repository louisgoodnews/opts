//! Redaction: a serialised profile contains only closed vocabulary, numbers and booleans, so no
//! MAC address, UUID, host name or user name can be in it, even when the renderer or the
//! operating system hands the probe hostile strings.

mod hostile;
mod patterns;
mod vocabulary;

use opts_hardware::inputs::ProbeInputs;
use opts_hardware::{probe_with, HardwareProfile};
use serde_json::Value;

use hostile::hostile;
use patterns::{has_mac, has_uuid};
use vocabulary::{allowed_words, assert_closed_vocabulary};

fn identity_words() -> Vec<String> {
    let mut v = Vec::new();
    for key in [
        "USER",
        "USERNAME",
        "LOGNAME",
        "HOSTNAME",
        "COMPUTERNAME",
        "HOME",
    ] {
        if let Ok(x) = std::env::var(key) {
            v.push(x);
        }
    }
    for f in ["/proc/sys/kernel/hostname", "/etc/hostname"] {
        if let Ok(x) = std::fs::read_to_string(f) {
            v.push(x.trim().to_owned());
        }
    }
    // Very short names ("a", "pc") would match innocent words; the structural test below covers them.
    v.retain(|w| w.len() >= 4);
    v
}

fn check(p: &HardwareProfile) {
    let text = p.to_json();
    assert!(!has_mac(&text), "MAC pattern in\n{text}");
    assert!(!has_uuid(&text), "UUID pattern in\n{text}");
    for w in identity_words() {
        assert!(!text.contains(&w), "identity word {w:?} in\n{text}");
    }
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_closed_vocabulary(&v, &allowed_words());
}

#[test]
fn real_profile_contains_no_identifiers() {
    check(&probe_with(&ProbeInputs::default()));
}

#[test]
fn profile_with_hostile_renderer_strings_contains_no_identifiers() {
    let p = probe_with(&hostile());
    check(&p);
    assert_eq!(
        p.gpu.features.known().map(Vec::len),
        Some(1),
        "only the allowlisted feature survives"
    );
    assert_eq!(p.display.refresh_hz.get(), Some(144));
}

#[test]
fn synthetic_and_default_profiles_contain_no_identifiers() {
    check(&HardwareProfile::default());
    let mut p = probe_with(&hostile());
    p.bench.measured_for_class = opts_hardware::Fact::Known(p.class_hash());
    check(&p);
}

#[test]
fn the_scanners_detect_what_they_should() {
    assert!(has_mac("x aa:bb:cc:dd:ee:ff y"));
    assert!(has_mac("AA-BB-CC-DD-EE-FF"));
    assert!(!has_mac("aa:bb:cc"));
    assert!(has_uuid(
        "{\"id\":\"123e4567-e89b-12d3-a456-426614174000\"}"
    ));
    assert!(!has_uuid("not-a-uuid-at-all"));
}
