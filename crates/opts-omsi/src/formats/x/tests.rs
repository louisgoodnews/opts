//! Unit tests for DirectX `.x` format parser.

use super::*;
use glam::{Vec2, Vec3};

#[test]
fn test_parse_self_authored_quad_mesh() {
    let sample = b"xof 0303txt 0032\r\n\
Frame Root {\r\n\
  FrameTransformMatrix {\r\n\
    1.0, 0.0, 0.0, 0.0,\r\n\
    0.0, 1.0, 0.0, 0.0,\r\n\
    0.0, 0.0, 1.0, 0.0,\r\n\
    0.0, 0.0, 0.0, 1.0;;\r\n\
  }\r\n\
  Mesh Quad {\r\n\
    4;\r\n\
    0.0; 0.0; 0.0;,\r\n\
    2.0; 0.0; 0.0;,\r\n\
    2.0; 2.0; 0.0;,\r\n\
    0.0; 2.0; 0.0;;\r\n\
    1;\r\n\
    4; 0, 1, 2, 3;;\r\n\
    MeshMaterialList {\r\n\
      1;\r\n\
      1;\r\n\
      0;\r\n\
      Material M {\r\n\
        1.0; 1.0; 1.0; 1.0;;\r\n\
        3.2;\r\n\
        0.0; 0.0; 0.0;;\r\n\
        0.0; 0.0; 0.0;;\r\n\
        TextureFilename { \"uvtest.bmp\"; }\r\n\
      }\r\n\
    }\r\n\
    MeshTextureCoords {\r\n\
      4;\r\n\
      0.0; 1.0;,\r\n\
      1.0; 1.0;,\r\n\
      1.0; 0.0;,\r\n\
      0.0; 0.0;;\r\n\
    }\r\n\
  }\r\n\
}\r\n";

    let mesh = parse_x(sample).expect("valid sample should parse");
    assert_eq!(mesh.frames.len(), 1);

    let root_frame = &mesh.frames[0];
    assert_eq!(root_frame.name.as_deref(), Some("Root"));
    assert!(root_frame.transform.is_some());

    let submesh = &root_frame.meshes[0];
    assert_eq!(submesh.name.as_deref(), Some("Quad"));
    assert_eq!(submesh.vertices.len(), 4);
    assert_eq!(submesh.vertices[0].position, Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(submesh.vertices[1].position, Vec3::new(2.0, 0.0, 0.0));
    assert_eq!(submesh.vertices[0].uv, Vec2::new(0.0, 1.0));

    // Quad must be triangulated into 2 triangles
    assert_eq!(submesh.indices.len(), 2);
    assert_eq!(submesh.indices[0], [0, 1, 2]);
    assert_eq!(submesh.indices[1], [0, 2, 3]);
    assert_eq!(submesh.face_materials, vec![0, 0]);

    // Material
    assert_eq!(submesh.materials.len(), 1);
    let mat = &submesh.materials[0];
    assert_eq!(mat.name.as_deref(), Some("M"));
    assert_eq!(mat.diffuse, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(mat.power, 3.2);
    assert_eq!(mat.texture_filename.as_deref(), Some("uvtest.bmp"));
}

#[test]
fn test_parse_named_material_reference() {
    let sample = b"xof 0303txt 0032\r\n\
Material GlobalMat {\r\n\
  0.5; 0.5; 0.5; 1.0;;\r\n\
  10.0;\r\n\
  1.0; 1.0; 1.0;;\r\n\
  0.0; 0.0; 0.0;;\r\n\
  TextureFilename { \"shared.png\"; }\r\n\
}\r\n\
Mesh Tri {\r\n\
  3;\r\n\
  0.0; 0.0; 0.0;,\r\n\
  1.0; 0.0; 0.0;,\r\n\
  0.0; 1.0; 0.0;;\r\n\
  1;\r\n\
  3; 0, 1, 2;;\r\n\
  MeshMaterialList {\r\n\
    1; 1; 0;\r\n\
    { GlobalMat }\r\n\
  }\r\n\
}\r\n";

    let mesh = parse_x(sample).expect("named material ref should resolve");
    assert_eq!(mesh.top_level_meshes.len(), 1);
    let submesh = &mesh.top_level_meshes[0];
    assert_eq!(submesh.materials.len(), 1);
    assert_eq!(submesh.materials[0].name.as_deref(), Some("GlobalMat"));
    assert_eq!(
        submesh.materials[0].texture_filename.as_deref(),
        Some("shared.png")
    );
}

#[test]
fn test_unresolved_material_reference() {
    let sample = b"xof 0303txt 0032\r\n\
Mesh Tri {\r\n\
  3;\r\n\
  0.0; 0.0; 0.0;, 1.0; 0.0; 0.0;, 0.0; 1.0; 0.0;;\r\n\
  1;\r\n\
  3; 0, 1, 2;;\r\n\
  MeshMaterialList {\r\n\
    1; 1; 0;\r\n\
    { MissingMat }\r\n\
  }\r\n\
}\r\n";

    let err = parse_x(sample).unwrap_err();
    assert_eq!(
        err,
        XParseError::UnresolvedMaterialReference("MissingMat".to_string())
    );
}

#[test]
fn test_skip_unknown_template() {
    let sample = b"xof 0303txt 0032\r\n\
template DeclData {\r\n\
  <3D82AB44-62DA-11CF-AB39-0020AF71E433>\r\n\
  DWORD nElements;\r\n\
}\r\n\
Mesh Tri {\r\n\
  3;\r\n\
  0.0; 0.0; 0.0;, 1.0; 0.0; 0.0;, 0.0; 1.0; 0.0;;\r\n\
  1;\r\n\
  3; 0, 1, 2;;\r\n\
}\r\n";

    let mesh = parse_x(sample).expect("should skip template and parse mesh");
    assert_eq!(mesh.top_level_meshes.len(), 1);
    assert_eq!(mesh.top_level_meshes[0].vertices.len(), 3);
}
