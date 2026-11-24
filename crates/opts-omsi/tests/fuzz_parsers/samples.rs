//! Small valid samples of the four text formats. No game file is read.

/// A quad with normals, texture coordinates and a material list: every count-bearing block.
pub const X: &str = "xof 0303txt 0032\n\
Frame Root {\n  Mesh Quad {\n    4;\n\
    0.0; 0.0; 0.0;,\n    2.0; 0.0; 0.0;,\n    2.0; 2.0; 0.0;,\n    0.0; 2.0; 0.0;;\n\
    2;\n    3; 0, 1, 2;,\n    3; 0, 2, 3;;\n\
    MeshNormals {\n      4;\n\
      0.0; 0.0; 1.0;,\n      0.0; 0.0; 1.0;,\n      0.0; 0.0; 1.0;,\n      0.0; 0.0; 1.0;;\n\
      2;\n      3; 0, 1, 2;,\n      3; 0, 2, 3;;\n    }\n\
    MeshTextureCoords {\n      4;\n\
      0.0; 0.0;,\n      1.0; 0.0;,\n      1.0; 1.0;,\n      0.0; 1.0;;\n    }\n\
    MeshMaterialList {\n      1;\n      2;\n      0, 0;;\n    }\n  }\n}\n";

pub const BUS: &str =
    "[friendlyname]\nMAN\nNL202\nBVG Standard\n\n[description]\nStandard\n[end]\n\
[coupling_front]\n0\n4.5\n0.3\n\n[coupling_front_character]\n52.5\n-20\n20\n1\n";

pub const SLI: &str = "[profilepnt]\n-2\n0.1\n0.5\n0.2\n[profilepnt]\n2\n0.1\n0.5\n0.2\n";

pub const HOF: &str =
    "[name]\nDemo Depot\n\n[global_strings]\n3\nDemo_Announcements\nDemo_Rollband\n\
Demo_Sidesign\n\n[addbusstop_list]\nHbf\tHBF\tHauptbahnhof\t\tHauptbahnhof\n[end]\n";

/// A minimal version 14 map tile: an object (no string values), a spline and the terrain flag.
pub const MAP_TILE: &str = r#"
File created with test

[version]
14

[terrain]

[object]
0
Sceneryobjects\Example\house.sco
100001
150.0
120.0
5.0
90.0
0.0
0.0
0

[spline]
0
Splines\Example\road.sli
200001
0
0
10.0
2.5
20.0
180.0
50.0
0.0
0
0
0
0
0.0
0.0
0
"#;

/// A version 3 `.o3d` mesh built in code: a triangle with 32-bit indices, one textured material
/// and the optional 4x4 matrix chunk. Layout as `o3d/parse.rs` reads it.
pub fn o3d() -> Vec<u8> {
    let mut b = vec![0x84, 0x19, 0x03, 0x01];
    // Vertex chunk (0x17): u32 count, then 32 bytes per vertex (position, normal, uv).
    b.push(0x17);
    b.extend(3u32.to_le_bytes());
    let vertices: [[f32; 8]; 3] = [
        [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        [2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0],
        [0.0, 2.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0],
    ];
    for v in vertices {
        for f in v {
            b.extend(f.to_le_bytes());
        }
    }
    // Face chunk (0x49): u32 count, then 3 u32 indices and a u16 material per face.
    b.push(0x49);
    b.extend(1u32.to_le_bytes());
    for i in [0u32, 1, 2] {
        b.extend(i.to_le_bytes());
    }
    b.extend(0u16.to_le_bytes());
    // Material chunk (0x26): u16 count; each record is 11 f32 (44 bytes), a u8 name length, name.
    b.push(0x26);
    b.extend(1u16.to_le_bytes());
    for f in [1.0f32, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 10.0] {
        b.extend(f.to_le_bytes());
    }
    b.push(8);
    b.extend(b"wall.bmp");
    // Optional transform matrix (0x79): 16 f32, identity.
    b.push(0x79);
    for f in [
        1.0f32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ] {
        b.extend(f.to_le_bytes());
    }
    b
}

/// A `.map.terrain` grid built in code: 2 cells per edge, so 3 x 3 heights after the u32 count.
pub fn terrain() -> Vec<u8> {
    let mut b = 2u32.to_le_bytes().to_vec();
    for i in 0..9 {
        b.extend((i as f32 * 0.5).to_le_bytes());
    }
    b
}
