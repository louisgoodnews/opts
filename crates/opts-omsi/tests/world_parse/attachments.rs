//! Spline attachments, text textures and spline helpers of `.sco` and `.map` files.

use super::*;

#[test]
fn spline_attachments_parse_single_and_repeater() {
    let text = "[version]\n14\n\n[splineAttachement]\n0\nSceneryobjects\\a.sco\n77\n3\n-4.5\n0.25\n12.5\n180\n0\n0\n30\n500\n0\n0\n\n[splineAttachement_repeater]\n0\n260\n6\nSceneryobjects\\b.sco\n78\n1\n9.5\n0.1\n8.8\n0\n0\n0\n30\n500\n0\n";
    let tile = opts_omsi::formats::map::MapTile::from_text(text).unwrap();
    assert_eq!(tile.spline_attachments.len(), 2);
    let single = &tile.spline_attachments[0];
    assert_eq!(
        (single.spline_index, single.lateral_m, single.distance_m),
        (3, -4.5, 12.5)
    );
    assert_eq!((single.spacing_m, single.range_m), (30.0, 500.0));
    assert_eq!(single.repeater, None);
    assert_eq!(single.instances(), 0..=16);
    let rep = &tile.spline_attachments[1];
    assert_eq!(
        rep.repeater,
        Some(opts_omsi::formats::map::RepeaterLink {
            master_tile: 260,
            first_instance: 6
        })
    );
    assert_eq!(rep.instances(), 6..=16);
    assert_eq!(
        (rep.spline_index, rep.sco_file.as_str()),
        (1, "Sceneryobjects\\b.sco")
    );
}

#[test]
fn text_texture_binds_to_the_preceding_material() {
    let text = "[texttexture_enh]\n1\nStreetBerlin\n256\n32\n0\n10\n20\n30\n2\n0\n\n[mesh]\nplate.o3d\n\n[matl]\nTextfeld_1.bmp\n0\n\n[useTextTexture]\n0\n\n[matl_alpha]\n2\n\n[matl]\nother.bmp\n1\n";
    let sco = SceneryObject::from_text(text).expect("parse");
    let tt = &sco.text_textures[0];
    assert_eq!(
        (tt.string_index, tt.orientation, tt.text_color),
        (1, 2, [10, 20, 30])
    );
    let mats = &sco.meshes[0].materials;
    assert_eq!(mats[0].text_texture, Some(0));
    assert_eq!(mats[0].alpha_mode, Some(2));
    assert_eq!(mats[1].text_texture, None);
}

#[test]
fn spline_helper_lines_are_x_height_y_and_crossings_are_absolute() {
    // Lines after the profile: x, height, y, heading, two further values (Einmuedung_Gewerbegebiet.sco).
    let text = "[splinehelper]\nSplines\\a.sli\n-12\n0.5\n-3\n270\n0\n0\n\n[mesh]\nm.o3d\n";
    let sco = SceneryObject::from_text(text).expect("parse");
    let h = &sco.spline_helpers[0];
    assert_eq!(
        (h.position.x, h.position.y, h.position.z),
        (-12.0, -3.0, 0.5)
    );
    assert_eq!(h.rotation_deg, 270.0);
    assert!(!sco.abs_height, "no [absheight]");
    assert!(
        sco.has_absolute_height(),
        "a crossing is stored at absolute height"
    );
    let plain = SceneryObject::from_text("[mesh]\nm.o3d\n").expect("parse");
    assert!(!plain.has_absolute_height());
}
