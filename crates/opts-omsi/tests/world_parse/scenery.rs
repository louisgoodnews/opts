//! `.sco` scenery objects of the world parse tests (`opts_omsi::formats::scenery::sco`).

use glam::Vec3;
use opts_omsi::formats::scenery::SceneryObject;

#[test]
fn test_scenery_object_full_feature_parsing() {
    let sco_text = r#"
[groups]
3
Berlin-Spandau
Streetobjects
Signs

[friendlyname]
Haltestelle_Mast_Standard

[complexity]
1

[fixed]
[surface]
[absheight]

[mesh]
Mast.o3d

[shadow]

[matl]
mast_tex.bmp
0

[matl_alpha]
1

[matl_envmap]
envmap.bmp
0.25

[matl_lightmap]
mast_lm.bmp

[mesh]
Schild.o3d

[visible]
show_sign
1

[usetexttexture]
0

[boundingbox]
0.4
0.4
3.0
0.0
0.0
1.5

[cog]
0.0
0.0
1.0

[crashmode_pole]
0.05
0.7

[texttexture]
0
DIN
256
64
0
255
255
255

[ctc]
skin_var
Texture\Skins
0

[ctctexture]
BVG
Texture\Skins\bvg.bmp

[script]
1
script\sign.osc

[varnamelist]
1
script\sign_varlist.txt

[stringvarnamelist]
1
script\sign_strlist.txt

[constfile]
1
script\sign_const.txt

[sound]
sound\sign.cfg

[passengercabin]
model\passengercabin.cfg
"#;

    let sco = SceneryObject::from_text(sco_text).expect("SceneryObject should parse");
    assert_eq!(sco.groups, vec!["Berlin-Spandau", "Streetobjects", "Signs"]);
    assert_eq!(
        sco.friendly_name.as_deref(),
        Some("Haltestelle_Mast_Standard")
    );
    assert_eq!(sco.complexity, Some(1));
    assert!(sco.fixed);
    assert!(sco.surface);
    assert!(sco.abs_height);

    assert_eq!(sco.meshes.len(), 2);
    assert_eq!(sco.meshes[0].file, "Mast.o3d");
    assert_eq!(sco.meshes[0].cast_shadow, Some(true));
    assert_eq!(sco.meshes[0].materials.len(), 1);
    assert_eq!(sco.meshes[0].materials[0].texture, "mast_tex.bmp");
    assert_eq!(sco.meshes[0].materials[0].alpha_mode, Some(1));
    assert_eq!(
        sco.meshes[0].materials[0].env_map,
        Some(("envmap.bmp".to_string(), 0.25))
    );
    assert_eq!(
        sco.meshes[0].materials[0].light_map.as_deref(),
        Some("mast_lm.bmp")
    );

    assert_eq!(sco.meshes[1].file, "Schild.o3d");
    assert_eq!(sco.meshes[1].visible, Some(("show_sign".to_string(), 1.0)));
    assert_eq!(sco.meshes[1].text_texture_indices, vec![0]);

    let bbox = sco.bounding_box.expect("bounding box should exist");
    assert_eq!(bbox.half_extents, Vec3::new(0.4, 0.4, 3.0));
    assert_eq!(bbox.center, Vec3::new(0.0, 0.0, 1.5));
    assert_eq!(sco.cog, Some(Vec3::new(0.0, 0.0, 1.0)));
    assert_eq!(sco.crashmode_pole, Some((0.05, 0.7)));

    assert_eq!(sco.text_textures.len(), 1);
    assert_eq!(sco.text_textures[0].string_index, 0);
    assert_eq!(sco.text_textures[0].font_name, "DIN");
    assert_eq!(sco.text_textures[0].width, 256);
    assert_eq!(sco.text_textures[0].height, 64);
    assert_eq!(sco.text_textures[0].text_color, [255, 255, 255]);

    let ctc = sco.ctc.expect("CTC should exist");
    assert_eq!(ctc.var_name, "skin_var");
    assert_eq!(ctc.texture_dir, r"Texture\Skins");
    assert_eq!(ctc.default_index, 0);
    assert_eq!(
        sco.ctc_textures,
        vec![("BVG".to_string(), r"Texture\Skins\bvg.bmp".to_string())]
    );

    assert_eq!(sco.scripts, vec![r"script\sign.osc"]);
    assert_eq!(sco.varnamelists, vec![r"script\sign_varlist.txt"]);
    assert_eq!(sco.stringvarnamelists, vec![r"script\sign_strlist.txt"]);
    assert_eq!(sco.constfiles, vec![r"script\sign_const.txt"]);
    assert_eq!(sco.sound.as_deref(), Some(r"sound\sign.cfg"));
    assert_eq!(
        sco.passenger_cabin.as_deref(),
        Some(r"model\passengercabin.cfg")
    );
}
