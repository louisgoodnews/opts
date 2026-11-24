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
