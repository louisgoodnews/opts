//! Parsing that feeds the scenery runtime (`docs/src/architecture/scenery-runtime.md`): typed
//! `[newanim]`, light effects, light-map variables and `[varparent]` links. Self-written fixtures.

use opts_core::formats::map::MapTile;
use opts_core::formats::scenery::SceneryObject;

const SCO: &str = "\
[LOD]
0.2

[mesh]
hand.o3d

[matl]
hand.bmp
0

[matl_lightmap]
hand_lm.bmp
Glow

[newanim]
origin_from_mesh
anim_rot
hand_angle
1

[visible]
hand_on
2

[light_enh_2]
0
0.5
2
0
1
0
0
0
1
0
0
255
200
0
0.3
30
60
lamp
1
0.2
0
0
0.1

[LOD]
0

[mesh]
stub.o3d

[matl]
stub.bmp
0

[matl_lightmap]
stub_lm.bmp

[newanim]
anim_rot
broken
not_a_number
";

#[test]
fn newanim_is_typed_and_malformed_blocks_become_warnings() {
    let sco = SceneryObject::from_text(SCO).expect("parses");
    let hand = &sco.meshes[0];
    assert_eq!(hand.animations.len(), 1);
    let anim = &hand.animations[0];
    assert!(anim.origin_from_mesh);
    let rot = anim.anim_rot.as_ref().expect("anim_rot");
    assert_eq!((rot.variable.as_str(), rot.factor), ("hand_angle", 1.0));
    assert_eq!(hand.visible, Some(("hand_on".to_string(), 2.0)));
    assert!(sco.meshes[1].animations.is_empty());
    assert_eq!(sco.warnings.len(), 1, "{:?}", sco.warnings);
}

#[test]
fn lights_remember_their_mesh_and_lod_group() {
    let sco = SceneryObject::from_text(SCO).expect("parses");
    assert_eq!(sco.lights.len(), 1);
    let l = &sco.lights[0];
    assert_eq!(l.variable, "lamp");
    assert_eq!(l.after_mesh, Some(0));
    assert_eq!(l.lod_threshold, Some(0.2));
    assert_eq!(l.color, [255.0, 200.0, 0.0]);
}

#[test]
fn light_map_variables_are_read_when_present() {
    let sco = SceneryObject::from_text(SCO).expect("parses");
    let m0 = &sco.meshes[0].materials[0];
    assert_eq!(m0.light_map.as_deref(), Some("hand_lm.bmp"));
    assert_eq!(m0.light_map_variable.as_deref(), Some("Glow"));
    // The second map names no variable (it follows the night lighting).
    let m1 = &sco.meshes[1].materials[0];
    assert_eq!(m1.light_map.as_deref(), Some("stub_lm.bmp"));
    assert_eq!(m1.light_map_variable, None);
}

const TILE: &str = "\
[version]
14

[object]
0
Sceneryobjects\\Junction.sco
19
10
20
0
0
0
0
0

[object]
0
Sceneryobjects\\Head.sco
195
12
22
2.8
149
0
0
1
2

[varparent]
19

[attachObj]
0
Sceneryobjects\\Plate.sco
300
195

[varparent]
19
";

#[test]
fn varparent_links_the_preceding_object_only() {
    let tile = MapTile::from_text(TILE).expect("parses");
    assert_eq!(tile.objects.len(), 2);
    assert_eq!(tile.var_parents, vec![(1, 19)]);
    assert_eq!(tile.objects[1].string_vars, vec!["2".to_string()]);
}
