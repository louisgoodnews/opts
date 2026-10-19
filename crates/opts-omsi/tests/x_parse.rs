//! Integration and robustness tests for DirectX `.x` parser.
//!
//! Validates parser against clean-room mutation kit fixtures (tests T10–T12)
//! located in the OMSI installation (`$OMSI_PATH`).

use std::path::PathBuf;

use opts_omsi::formats::x::{parse_x, XParseError};

/// The mutation-test fixtures live in the OMSI installation (`Sceneryobjects/OPTS_Test/model`),
/// found through `$OPTS_X_FIXTURES` (the folder itself) or `$OMSI_PATH` (the installation).
/// Without them the tests skip.
fn fixtures_dir() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = std::env::var_os("OPTS_X_FIXTURES") {
        candidates.push(PathBuf::from(p));
    }
    if let Some(root) = std::env::var_os("OMSI_PATH") {
        candidates.push(PathBuf::from(root).join("Sceneryobjects/OPTS_Test/model"));
    }
    candidates
        .into_iter()
        .find(|p| p.join("t10_x_ident_quad.x").exists())
}

fn read_fixture(filename: &str) -> Option<Vec<u8>> {
    let Some(dir) = fixtures_dir() else {
        eprintln!("OPTS_Test fixtures not found (set OMSI_PATH); skipping");
        return None;
    };
    let path = dir.join(filename);
    Some(
        std::fs::read(&path)
            .unwrap_or_else(|e| panic!("Failed to read fixture {}: {}", path.display(), e)),
    )
}

#[test]
fn test_t10_quad_triangulation_vs_triangles() {
    let Some(quad_bytes) = read_fixture("t10_x_ident_quad.x") else {
        return;
    };
    let Some(tri_bytes) = read_fixture("t10_x_ident_tri.x") else {
        return;
    };

    let quad_mesh = parse_x(&quad_bytes).expect("t10_x_ident_quad.x should parse");
    let tri_mesh = parse_x(&tri_bytes).expect("t10_x_ident_tri.x should parse");

    let q_sub = quad_mesh.primary_mesh().expect("should have submesh");
    let t_sub = tri_mesh.primary_mesh().expect("should have submesh");

    // Both should produce exactly 4 vertices and 2 triangulated faces
    assert_eq!(q_sub.vertices.len(), 4);
    assert_eq!(t_sub.vertices.len(), 4);
    assert_eq!(q_sub.indices.len(), 2);
    assert_eq!(t_sub.indices.len(), 2);

    // Quad triangulation produces [0, 1, 2] and [0, 2, 3] matching t10_x_ident_tri
    assert_eq!(q_sub.indices[0], [0, 1, 2]);
    assert_eq!(q_sub.indices[1], [0, 2, 3]);
    assert_eq!(t_sub.indices[0], [0, 1, 2]);
    assert_eq!(t_sub.indices[1], [0, 2, 3]);

    // Material slot 0 assigned to both triangles
    assert_eq!(q_sub.face_materials, vec![0, 0]);
    assert_eq!(t_sub.face_materials, vec![0, 0]);
}

#[test]
fn test_t11_frame_transform_matrices() {
    // 1. SwapYZ: row 1 is (0,0,1,0), row 2 is (0,1,0,0)
    let Some(swap_bytes) = read_fixture("t11_x_swapyz.x") else {
        return;
    };
    let swap_mesh = parse_x(&swap_bytes).expect("t11_x_swapyz.x should parse");
    let swap_frame = &swap_mesh.frames[0];
    let m = swap_frame.transform.expect("frame must have transform");
    assert_eq!(m[1], [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(m[2], [0.0, 1.0, 0.0, 0.0]);

    // 2. Scale 0.025: diagonal is (0.025, 0.025, 0.025, 1.0)
    let Some(scale_bytes) = read_fixture("t11_x_scale0025.x") else {
        return;
    };
    let scale_mesh = parse_x(&scale_bytes).expect("t11_x_scale0025.x should parse");
    let scale_frame = &scale_mesh.frames[0];
    let m_scale = scale_frame.transform.expect("frame must have transform");
    assert!((m_scale[0][0] - 0.025).abs() < 1e-6);
    assert!((m_scale[1][1] - 0.025).abs() < 1e-6);
    assert!((m_scale[2][2] - 0.025).abs() < 1e-6);

    // 3. Translate 3.0 in X: m[3] is (3.0, 0.0, 0.0, 1.0)
    let Some(trans_bytes) = read_fixture("t11_x_translate3.x") else {
        return;
    };
    let trans_mesh = parse_x(&trans_bytes).expect("t11_x_translate3.x should parse");
    let trans_frame = &trans_mesh.frames[0];
    let m_trans = trans_frame.transform.expect("frame must have transform");
    assert!((m_trans[3][0] - 3.0).abs() < 1e-6);
}

#[test]
fn test_t12_named_material_reference_resolution() {
    let Some(ref_bytes) = read_fixture("t12_x_refmaterial.x") else {
        return;
    };
    let ref_mesh = parse_x(&ref_bytes).expect("t12_x_refmaterial.x should parse");

    // Top-level material M was defined
    assert_eq!(ref_mesh.global_materials.len(), 1);
    assert_eq!(ref_mesh.global_materials[0].name.as_deref(), Some("M"));

    // Submesh references material M
    let sub = ref_mesh.primary_mesh().expect("primary mesh should exist");
    assert_eq!(sub.materials.len(), 1);
    assert_eq!(sub.materials[0].name.as_deref(), Some("M"));
    assert_eq!(
        sub.materials[0].texture_filename.as_deref(),
        Some("uvtest.bmp")
    );
}

#[test]
fn test_t12_reversed_face_winding() {
    let Some(rev_bytes) = read_fixture("t12_x_reversed.x") else {
        return;
    };
    let rev_mesh = parse_x(&rev_bytes).expect("t12_x_reversed.x should parse");
    let sub = rev_mesh.primary_mesh().expect("primary mesh should exist");

    // Face 4; 0, 3, 2, 1 splits into [0, 3, 2] and [0, 2, 1]
    assert_eq!(sub.indices[0], [0, 3, 2]);
    assert_eq!(sub.indices[1], [0, 2, 1]);
}

#[test]
fn test_robustness_empty_and_short_inputs() {
    assert_eq!(parse_x(b""), Err(XParseError::TruncatedHeader(0)));
    assert_eq!(parse_x(b"xof "), Err(XParseError::TruncatedHeader(4)));
    // An empty binary payload is a valid (empty) file; compressed variants are rejected.
    assert!(parse_x(b"xof 0303bin 0032").is_ok());
    assert_eq!(
        parse_x(b"xof 0303tzip0032"),
        Err(XParseError::UnsupportedFormat("tzip".to_string()))
    );
    assert_eq!(
        parse_x(b"not_directx_header"),
        Err(XParseError::InvalidMagic(*b"not_"))
    );
}

#[test]
fn test_robustness_pseudorandom_fuzzing() {
    // 1,000 fuzzed inputs with valid header + random payloads must never panic
    let mut state: u64 = 0xDEADBEEFCAFEBABE;
    let mut next_u32 = || -> u32 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state & 0xFFFF_FFFF) as u32
    };

    let mut buf = Vec::with_capacity(1024);
    for _ in 0..1000 {
        buf.clear();
        buf.extend_from_slice(b"xof 0303txt 0032\r\n");
        let payload_len = (next_u32() % 256) as usize;
        for _ in 0..payload_len {
            buf.push((next_u32() & 0xFF) as u8);
        }

        // Never panic: returns Ok(_) or Err(_)
        let _ = parse_x(&buf);
    }
}
