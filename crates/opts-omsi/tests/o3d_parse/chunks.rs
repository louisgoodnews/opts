//! Matrix chunks and per-triangle material indices of the mutation kit.

use super::*;

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
