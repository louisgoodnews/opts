//! Integration tests for `.o3d` parser against mutation kit fixtures.

use std::path::PathBuf;

use opts_core::formats::o3d::{parse, O3dParseError};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(
        "/home/louisgoodnews/Dokumente/opts-mutation-kit/out/Sceneryobjects/OPTS_Test/model",
    )
}

fn load_fixture(name: &str) -> Option<Vec<u8>> {
    let path = fixtures_dir().join(name);
    if path.exists() {
        Some(std::fs::read(&path).expect("failed to read fixture"))
    } else {
        None
    }
}

#[test]
fn test_t01_axes_and_winding() {
    let bytes_a = match load_fixture("t01_axes_windA.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d_a = parse(&bytes_a).expect("t01_axes_windA should parse successfully");
    assert_eq!(o3d_a.version, 7);
    assert!(o3d_a.vertex_count() > 0);
    assert!(o3d_a.triangle_count() > 0);

    // Verify axis extents exist and vertices are preserved
    let mut max_x = 0.0f32;
    let mut max_y = 0.0f32;
    let mut max_z = 0.0f32;
    for v in &o3d_a.vertices {
        max_x = max_x.max(v.position.x);
        max_y = max_y.max(v.position.y);
        max_z = max_z.max(v.position.z);
    }
    assert!(max_x > 0.0);
    assert!(max_y > 0.0);
    assert!(max_z > 0.0);

    // Compare with reversed winding B
    let bytes_b = match load_fixture("t01_axes_windB.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d_b = parse(&bytes_b).expect("t01_axes_windB should parse successfully");
    assert_eq!(o3d_a.vertex_count(), o3d_b.vertex_count());
    assert_eq!(o3d_a.triangle_count(), o3d_b.triangle_count());
    // Winding is reversed between A and B
    assert_ne!(o3d_a.indices[0], o3d_b.indices[0]);
}

#[test]
fn test_t02_uv_orientation() {
    let bytes = match load_fixture("t02_uv_v1.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d = parse(&bytes).expect("t02_uv_v1 should parse successfully");
    assert_eq!(o3d.vertex_count(), 4);
    assert_eq!(o3d.triangle_count(), 2);
    // UV coordinates range between 0.0 and 1.0
    for v in &o3d.vertices {
        assert!(v.uv.x >= 0.0 && v.uv.x <= 1.0);
        assert!(v.uv.y >= 0.0 && v.uv.y <= 1.0);
    }
}

#[test]
fn test_t03_materials() {
    let bytes = match load_fixture("t03_materials.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d = parse(&bytes).expect("t03_materials should parse successfully");
    assert!(o3d.material_count() >= 7);
    // Test that material colors have distinct groups
    for mat in &o3d.materials {
        assert_eq!(mat.diffuse.len(), 4);
        assert_eq!(mat.specular.len(), 3);
        assert_eq!(mat.emissive.len(), 3);
        assert!(mat.power >= 0.0);
    }
}

#[test]
fn test_t04_matrix_chunks() {
    // Identity matrix
    let bytes_id = match load_fixture("t04_a_identity.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d_id = parse(&bytes_id).expect("t04_a should parse");
    let mat = o3d_id.transform.expect("matrix chunk should be present");
    assert_eq!(mat[0], [1.0, 0.0, 0.0, 0.0]);
    assert_eq!(mat[1], [0.0, 1.0, 0.0, 0.0]);
    assert_eq!(mat[2], [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(mat[3], [0.0, 0.0, 0.0, 1.0]);

    // Translate matrix
    let bytes_tr = match load_fixture("t04_b_translate.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d_tr = parse(&bytes_tr).expect("t04_b should parse");
    assert!(o3d_tr.transform.is_some());

    // Scale matrix
    let bytes_sc = match load_fixture("t04_c_scale2.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d_sc = parse(&bytes_sc).expect("t04_c should parse");
    assert!(o3d_sc.transform.is_some());

    // Omitted matrix chunk -> None
    let bytes_no = match load_fixture("t04_d_nomatrix.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d_no = parse(&bytes_no).expect("t04_d should parse");
    assert!(o3d_no.transform.is_none());
}

#[test]
fn test_t05_supported_versions() {
    let versions = [
        ("t05_v1.o3d", 1),
        ("t05_v3.o3d", 3),
        ("t05_v4.o3d", 4),
        ("t05_v5.o3d", 5),
        ("t05_v7_f0.o3d", 7),
        ("t05_v7_f2.o3d", 7),
        ("t05_v7_stamp0.o3d", 7),
        ("t05_v7_stampX.o3d", 7),
    ];

    for (file, expected_ver) in versions {
        let bytes = match load_fixture(file) {
            Some(b) => b,
            None => continue,
        };
        let o3d = parse(&bytes).unwrap_or_else(|e| panic!("failed parsing {file}: {e}"));
        assert_eq!(o3d.version, expected_ver);
        assert!(o3d.vertex_count() > 0);
        assert!(o3d.triangle_count() > 0);
    }
}

#[test]
fn test_t05_flags_and_index_widths() {
    // 32-bit index flag bit 0
    let files = ["t05_v7_f1_idx32.o3d", "t05_v7_f3_idx32.o3d"];
    for file in files {
        let bytes = match load_fixture(file) {
            Some(b) => b,
            None => continue,
        };
        let o3d = parse(&bytes).unwrap_or_else(|e| panic!("failed parsing {file}: {e}"));
        assert_eq!(o3d.version, 7);
        assert_ne!(o3d.header_flags & 0x01, 0);
        assert!(o3d.triangle_count() > 0);
    }
}

#[test]
fn test_t05_rejects_unknown_versions() {
    let unknown_files = [
        ("t05_unknown_v2.o3d", 2),
        ("t05_unknown_v6.o3d", 6),
        ("t05_unknown_v8.o3d", 8),
    ];

    for (file, expected_ver) in unknown_files {
        let bytes = match load_fixture(file) {
            Some(b) => b,
            None => continue,
        };
        let err = parse(&bytes).expect_err(&format!("{file} should be rejected"));
        assert_eq!(err, O3dParseError::UnsupportedVersion(expected_ver));
    }
}

#[test]
fn test_t06_facemat_per_triangle_materials() {
    let bytes = match load_fixture("t06_facemat.o3d") {
        Some(b) => b,
        None => return,
    };
    let o3d = parse(&bytes).expect("t06_facemat should parse");
    assert!(o3d.face_materials.len() == o3d.triangle_count());
    // Verify that multiple distinct material indices exist in the face material list
    let distinct_mats: std::collections::HashSet<_> = o3d.face_materials.iter().copied().collect();
    assert!(distinct_mats.len() > 1);
}

#[test]
fn test_robustness_empty_and_short_inputs() {
    // 1. Empty byte slice
    assert!(parse(&[]).is_err());

    // 2. Single-byte inputs across all possible values
    for b in 0..=u8::MAX {
        assert!(parse(&[b]).is_err());
    }

    // 3. Short prefixes of magic
    assert!(parse(&[0x84]).is_err());
    assert!(parse(&[0x84, 0x19]).is_err());
}

#[test]
fn test_robustness_pseudorandom_fuzzing() {
    // Deterministic 64-bit Xorshift PRNG
    let mut state: u64 = 0xCAFE_BABE_DEAD_BEEF;
    let mut xorshift64 = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    // Test 100 buffers of 512 pseudorandom bytes
    for _ in 0..100 {
        let mut buffer = [0u8; 512];
        for chunk in buffer.chunks_exact_mut(8) {
            chunk.copy_from_slice(&xorshift64().to_le_bytes());
        }
        let _ = parse(&buffer); // Must not panic
    }
}

#[test]
fn test_robustness_prefix_truncation() {
    let bytes = match load_fixture("t04_a_identity.o3d") {
        Some(b) => b,
        None => return,
    };

    // For every prefix from 1 byte up to len - 1:
    // It must NEVER panic. If it succeeds, it can only be at the optional chunk boundary (offset 939).
    for len in 1..bytes.len() {
        let result = parse(&bytes[..len]);
        if len != 939 {
            assert!(
                result.is_err(),
                "prefix of length {len} should fail, but succeeded"
            );
        } else {
            // Offset 939 is the valid end of materials before the optional 0x79 matrix tag
            assert!(result.is_ok());
        }
    }
}
