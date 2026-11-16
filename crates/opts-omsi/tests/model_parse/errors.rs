//! Error reporting and fuzzed input of the `model.cfg` parser.

use super::*;

#[test]
fn test_missing_parameters_error() {
    let text = r#"
[mesh]
"#;
    let err = ModelConfig::parse(text).unwrap_err();
    assert!(matches!(err, ModelParseError::MissingParameters { .. }));
}

#[test]
fn test_invalid_number_error() {
    let text = r#"
[LOD]
not_a_number
"#;
    let err = ModelConfig::parse(text).unwrap_err();
    assert!(matches!(err, ModelParseError::InvalidNumber { .. }));
}

#[test]
fn test_unknown_keywords_preserved() {
    let text = r#"
[custom_global]
param_a
param_b

[mesh]
test.o3d

[custom_mesh_prop]
val1
val2
"#;
    let config = ModelConfig::parse(text).expect("Should preserve unknown keywords");
    assert!(config.unknown_fields.contains_key("custom_global"));
    let mesh = &config.lods[0].meshes[0];
    assert!(mesh.unknown_fields.contains_key("custom_mesh_prop"));
}

#[test]
fn test_fuzz_pseudo_random_buffers() {
    let mut state: u32 = 0xDEADBEEF;
    let mut next_u8 = || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        (state >> 24) as u8
    };

    for _ in 0..1000 {
        let len = (next_u8() as usize) % 256;
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(next_u8());
        }

        // Fuzzing with ModelConfig::parse_bytes must never panic
        let _ = ModelConfig::parse_bytes(&buf);
    }
}
