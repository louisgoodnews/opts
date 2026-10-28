//! Unit tests for `super` (`mesh.rs`).

mod finite_tests {
    use super::super::*;
    use bevy::math::{Vec2, Vec3};
    use opts_core::formats::o3d::Vertex;

    fn vertex(p: Vec3) -> Vertex {
        Vertex {
            position: p,
            normal: Vec3::Y,
            uv: Vec2::ZERO,
            color: None,
        }
    }

    #[test]
    fn faces_with_non_finite_vertices_are_skipped() {
        let o = O3d {
            version: 1,
            header_flags: 0,
            vertices: vec![
                vertex(Vec3::ZERO),
                vertex(Vec3::X),
                vertex(Vec3::Z),
                vertex(Vec3::new(f32::INFINITY, 0.0, 0.0)),
            ],
            indices: vec![[0, 1, 2], [0, 1, 3]],
            face_materials: vec![0, 0],
            materials: Vec::new(),
            transform: None,
            encryption_key: None,
            exact_decoded: false,
            unknown_chunks: Vec::new(),
        };
        let meshes = o3d_to_bevy_meshes_by_material(&o);
        assert_eq!(meshes.len(), 1);
        let n = meshes[0].1.indices().map_or(0, bevy::mesh::Indices::len);
        assert_eq!(n, 3, "only the finite triangle remains");
    }
}

mod x_palette_tests {
    use super::super::*;

    /// Two frames like `strschild_1.x`: a plate with `StrSchild_1.bmp` and a text quad with
    /// `StrSchild_Text1.bmp`, each with its own one-entry material list.
    pub(crate) const TWO_FRAME_SIGN: &[u8] = b"xof 0303txt 0032\n\
Frame Plate {\n\
  Mesh {\n\
    3; 0.0;0.0;0.0;, 1.0;0.0;0.0;, 0.0;1.0;0.0;;\n\
    1; 3; 0,1,2;;\n\
    MeshMaterialList { 1; 1; 0;\n\
      Material Mat1 { 1.0;1.0;1.0;1.0;; 0.0; 0.0;0.0;0.0;; 0.0;0.0;0.0;;\n\
        TextureFilename { \"StrSchild_1.bmp\"; } }\n\
    }\n\
  }\n\
}\n\
Frame Text {\n\
  Mesh {\n\
    3; 0.0;0.0;0.1;, 1.0;0.0;0.1;, 0.0;1.0;0.1;;\n\
    1; 3; 0,1,2;;\n\
    MeshMaterialList { 1; 1; 0;\n\
      Material Mat1 { 1.0;1.0;1.0;1.0;; 0.0; 0.0;0.0;0.0;; 0.0;0.0;0.0;;\n\
        TextureFilename { \"StrSchild_Text1.bmp\"; } }\n\
    }\n\
  }\n\
}\n";

    #[test]
    fn x_sub_meshes_share_the_file_wide_palette() {
        let models =
            parse_mesh_file(std::path::Path::new("strschild_1.x"), TWO_FRAME_SIGN).expect("parse");
        assert_eq!(models.len(), 2);
        for m in &models {
            let names: Vec<_> = m.materials.iter().map(|m| m.primary_texture()).collect();
            assert_eq!(
                names,
                vec![Some("StrSchild_1.bmp"), Some("StrSchild_Text1.bmp")]
            );
        }
        assert_eq!(models[0].face_materials, vec![0]);
        assert_eq!(models[1].face_materials, vec![1]);
    }

    #[test]
    fn text_override_does_not_reach_the_plate() {
        let models =
            parse_mesh_file(std::path::Path::new("strschild_1.x"), TWO_FRAME_SIGN).expect("parse");
        let over = [crate::model::TextureOverride {
            slot: 0,
            texture: "StrSchild_Text1.bmp".into(),
            alpha: Some(2),
            alpha_variable: None,
            trans_map: None,
            image: None,
            light_map: None,
            free_texture: None,
            night_map: None,
        }];
        let palette = &models[0].materials;
        assert!(crate::model::find_override(palette, 0, &over).is_none());
        assert!(crate::model::find_override(palette, 1, &over).is_some());
    }

    #[test]
    fn equal_materials_of_two_sub_meshes_stay_two_slots() {
        let doubled = std::str::from_utf8(TWO_FRAME_SIGN)
            .expect("utf8")
            .replace("StrSchild_Text1.bmp", "StrSchild_1.bmp");
        let models =
            parse_mesh_file(std::path::Path::new("a.x"), doubled.as_bytes()).expect("parse");
        assert_eq!(models[0].materials.len(), 2);
        assert_eq!(models[1].face_materials, vec![1]);
        // `[matl] StrSchild_1.bmp 1` reaches the second sub-mesh only (occurrence 1).
        let over = [crate::model::TextureOverride {
            slot: 1,
            texture: "StrSchild_1.bmp".into(),
            alpha: Some(1),
            alpha_variable: None,
            trans_map: None,
            image: None,
            light_map: None,
            free_texture: None,
            night_map: None,
        }];
        let palette = &models[0].materials;
        assert!(crate::model::find_override(palette, 0, &over).is_none());
        assert!(crate::model::find_override(palette, 1, &over).is_some());
    }
}

mod tree_tests {
    use super::super::*;

    #[test]
    fn billboard_has_two_crossed_quads() {
        let m = tree_billboard_mesh(16.0, 18.0);
        assert_eq!(m.count_vertices(), 8);
        assert_eq!(m.indices().map(bevy::mesh::Indices::len), Some(12));
    }

    #[test]
    fn billboard_normals_are_crown_normals() {
        let m = tree_billboard_mesh(16.0, 18.0);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            m.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(nrm)) =
            m.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals");
        };
        for (p, n) in pos.iter().zip(nrm) {
            let n = Vec3::from(*n);
            assert!((n.length() - 1.0).abs() < 1e-5);
            assert!((n.y - 0.8).abs() < 1e-5, "up component {n}");
            let outward = Vec3::new(p[0], 0.0, p[2]).normalize();
            assert!((Vec3::new(n.x, 0.0, n.z).dot(outward) - 0.6).abs() < 1e-5);
        }
    }

    #[test]
    fn billboard_material_draws_both_faces_without_normal_flip() {
        let m = tree_billboard_material(bevy::asset::Handle::default());
        assert!(!m.double_sided);
        assert_eq!(m.cull_mode, None);
        assert!(m.reflectance.abs() < f32::EPSILON);
        assert!(matches!(m.alpha_mode, bevy::prelude::AlphaMode::Mask(_)));
    }
}
