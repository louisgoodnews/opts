//! Tests of the foliage rule and the mesh-tree test.

use super::foliage::{is_foliage_texture, is_mesh_tree};
use opts_core::formats::scenery::SceneryObject;

#[test]
fn foliage_textures_are_told_from_look_alikes() {
    for name in [
        "Laub_01.tga",
        "tree_medium_07_leaves.dds",
        "Tannen.bmp",
        "Kastanienbaum.tga",
        "Birken2.dds",
        "Ziergras.tga",
        "Blätter.bmp",
        "hedge.tga",
    ] {
        assert!(is_foliage_texture(name), "{name}");
    }
    for name in [
        "SK_Bushokje.tga",
        "Lichtentanne_Gelaender.bmp",
        "Verkehrszeichen.tga",
        "Bark01.dds",
        "Glas.tga",
        "Oaktown_Sign.bmp",
        "Uhr_Zifferblatt.tga",
    ] {
        assert!(!is_foliage_texture(name), "{name}");
    }
}

fn object(materials: &[(&str, Option<u32>)]) -> SceneryObject {
    use opts_core::formats::scenery::ScoMaterial;
    let mut sco = SceneryObject::default();
    let mut mesh = opts_core::formats::scenery::ScoMesh {
        file: "tree.o3d".into(),
        lod_threshold: None,
        cast_shadow: None,
        materials: Vec::new(),
        visible: None,
        mouse_event: None,
        text_texture_indices: Vec::new(),
        animations: Vec::new(),
        only_editor: false,
    };
    for (i, (tex, alpha)) in materials.iter().enumerate() {
        let mut m = ScoMaterial::new(*tex, i);
        m.alpha_mode = *alpha;
        mesh.materials.push(m);
    }
    sco.meshes.push(mesh);
    sco
}

#[test]
fn an_object_is_a_tree_when_all_its_cut_outs_are_foliage() {
    assert!(is_mesh_tree(&object(&[
        ("Bark10.png", None),
        ("Branch10.png", Some(1))
    ])));
    assert!(!is_mesh_tree(&object(&[
        ("Wand.tga", None),
        ("busch_alpha.png", Some(1)),
        ("Fenster.tga", Some(2)),
    ])));
    assert!(
        !is_mesh_tree(&object(&[("Branch10.png", None)])),
        "opaque: no foliage"
    );
}
