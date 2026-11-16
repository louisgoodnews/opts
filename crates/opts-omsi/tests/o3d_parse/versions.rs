//! Supported and rejected format versions, flags and index widths of the mutation kit.

use super::*;

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
