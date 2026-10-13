//! Redaction: a serialised profile contains only closed vocabulary, numbers and booleans, so no
//! MAC address, UUID, host name or user name can be in it, even when the renderer or the
//! operating system hands the probe hostile strings.

use opts_hardware::inputs::{AudioInput, DisplayInput, GpuInput, InputDevicesInput, ProbeInputs};
use opts_hardware::probe::{GPU_FEATURES, ISA_FEATURES};
use opts_hardware::{probe_with, HardwareProfile, Reason};
use serde_json::Value;

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `aa:bb:cc:dd:ee:ff` or `aa-bb-cc-dd-ee-ff`.
fn has_mac(text: &str) -> bool {
    let b = text.as_bytes();
    (0..b.len().saturating_sub(16)).any(|i| {
        let w = &text[i..i + 17];
        w.is_ascii()
            && [2, 5, 8, 11, 14]
                .iter()
                .all(|k| matches!(w.as_bytes()[*k], b':' | b'-'))
            && [0, 3, 6, 9, 12, 15].iter().all(|k| is_hex(&w[*k..*k + 2]))
    })
}

/// 8-4-4-4-12 hex groups.
fn has_uuid(text: &str) -> bool {
    let b = text.as_bytes();
    (0..b.len().saturating_sub(35)).any(|i| {
        let w = &text[i..i + 36];
        w.is_ascii()
            && [8, 13, 18, 23].iter().all(|k| w.as_bytes()[*k] == b'-')
            && [(0, 8), (9, 13), (14, 18), (19, 23), (24, 36)]
                .iter()
                .all(|(a, z)| is_hex(&w[*a..*z]))
    })
}

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

fn allowed_words() -> Vec<String> {
    let mut v: Vec<String> = Reason::ALL.iter().map(|r| r.key().to_owned()).collect();
    v.extend(
        ISA_FEATURES
            .iter()
            .chain(GPU_FEATURES)
            .map(|s| (*s).to_owned()),
    );
    // Enum keys: take them from the generated vocabulary of the reference page.
    v.extend(
        opts_hardware::reference::render()
            .lines()
            .filter(|l| l.starts_with("* `"))
            .flat_map(|l| {
                l.split('`')
                    .skip(1)
                    .step_by(2)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            }),
    );
    v
}

/// Every string in the document is a field name, an enum key, a feature name, a reason or the
/// 16-digit class hash; everything else is a number, a bool or null.
fn assert_closed_vocabulary(v: &Value, allowed: &[String]) {
    match v {
        Value::String(s) => assert!(
            allowed.contains(s) || (s.len() == 16 && is_hex(s)),
            "string {s:?} is outside the vocabulary"
        ),
        Value::Array(a) => a.iter().for_each(|x| assert_closed_vocabulary(x, allowed)),
        Value::Object(m) => m
            .values()
            .for_each(|x| assert_closed_vocabulary(x, allowed)),
        _ => {}
    }
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

fn hostile() -> ProbeInputs {
    let evil = "AA:BB:CC:DD:EE:FF 123e4567-e89b-12d3-a456-426614174000 my-hostname louis";
    ProbeInputs {
        gpu: Some(GpuInput {
            vendor_id: 0x10de,
            device_type: format!("DiscreteGpu {evil}"),
            backend: format!("Vulkan {evil}"),
            vram_mib: Some(8192),
            max_texture_2d: Some(16384),
            max_buffer_bytes: Some(1 << 32),
            features: vec![
                evil.to_owned(),
                "texture_compression_bc".to_owned(),
                format!("{evil}timestamp_query"),
            ],
            ray_tracing: Some(true),
            dlss_runtime: Some(false),
        }),
        display: Some(DisplayInput {
            count: 2,
            width: 2560,
            height: 1440,
            refresh_millihertz: Some(143_981),
            scale_factor: Some(1.0),
            hdr: Some(false),
        }),
        audio: Some(AudioInput {
            device_present: true,
            channels: Some(2),
        }),
        input: Some(InputDevicesInput {
            gamepads: 1,
            wheels: Some(0),
        }),
        ..ProbeInputs::default()
    }
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
