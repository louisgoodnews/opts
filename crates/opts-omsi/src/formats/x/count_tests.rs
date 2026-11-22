//! Tests that file-supplied list counts are checked against the tokens left (ISS-005).

use super::*;

const QUAD: &str = "xof 0303txt 0032\n\
Frame Root {\n\
  Mesh Quad {\n\
    4;\n\
    0.0; 0.0; 0.0;,\n\
    2.0; 0.0; 0.0;,\n\
    2.0; 2.0; 0.0;,\n\
    0.0; 2.0; 0.0;;\n\
    1;\n\
    4; 0, 1, 2, 3;;\n\
  }\n\
}\n";

/// A count beyond the tokens left is rejected by `parse_count`, whose error names the tokens left.
/// Checking the message keeps these tests from passing through the end-of-file path when the bound
/// is missing.
fn assert_count_error(err: XParseError, what: &str) {
    match &err {
        XParseError::UnexpectedToken { expected, .. } if expected.contains("tokens left") => {}
        other => panic!("{what}: expected the count bound error, got {other:?}"),
    }
}

#[test]
fn a_valid_quad_with_matching_counts_still_parses() {
    let mesh = parse_x(QUAD.as_bytes()).expect("valid quad should parse");
    let submesh = &mesh.frames[0].meshes[0];
    assert_eq!(submesh.vertices.len(), 4);
    assert_eq!(submesh.indices.len(), 2, "a quad becomes two triangles");
}

#[test]
fn a_huge_vertex_count_is_an_error_not_an_allocation() {
    let text = QUAD.replacen("Mesh Quad {\n4;\n", "Mesh Quad {\n4000000000;\n", 1);
    let err = parse_x(text.as_bytes()).expect_err("a count beyond the file must fail");
    assert_count_error(err, "a count beyond the file must fail");
}

#[test]
fn a_huge_face_count_is_an_error_not_an_allocation() {
    let text = QUAD.replacen("\n1;\n", "\n4000000000;\n", 1);
    let err = parse_x(text.as_bytes()).expect_err("a face count beyond the file must fail");
    assert_count_error(err, "a count beyond the file must fail");
}

#[test]
fn a_huge_per_face_index_count_is_an_error_not_an_allocation() {
    let text = QUAD.replacen("4; 0, 1, 2, 3;;", "4000000000; 0, 1, 2, 3;;", 1);
    let err = parse_x(text.as_bytes()).expect_err("an index count beyond the file must fail");
    assert_count_error(err, "a count beyond the file must fail");
}
