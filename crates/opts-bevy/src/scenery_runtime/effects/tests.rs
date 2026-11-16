//! Tests of the scenery effects: a directional light's cone, and the visibility of a mesh rule.

use super::*;

fn light(direction: Vec3) -> SceneryLight {
    SceneryLight {
        var: VarRef::One,
        factor: 1.0,
        size_m: 0.3,
        direction,
        cos_inner: 20f32.to_radians().cos(),
        cos_outer: 50f32.to_radians().cos(),
        level: 1.0,
    }
}

#[test]
fn a_directional_light_fades_outside_its_cone() {
    let l = light(Vec3::Z);
    assert_eq!(l.seen_level(Vec3::Z), 1.0);
    assert_eq!(l.seen_level(-Vec3::Z), 0.0);
    let mid = Vec3::new(35f32.to_radians().sin(), 0.0, 35f32.to_radians().cos());
    let s = l.seen_level(mid);
    assert!(s > 0.1 && s < 0.9, "{s}");
    assert_eq!(light(Vec3::ZERO).seen_level(-Vec3::Z), 1.0);
}

#[test]
fn visibility_needs_equality_with_the_flag() {
    let rule = SceneryMeshRule {
        anim: MeshAnimation {
            owner: Entity::PLACEHOLDER,
            steps: Vec::new(),
            visible: vec![("season".into(), 0.0)],
            shown: Vec::new(),
        },
        vars: vec![VarRef::Host(HostVar::Zero)],
    };
    assert!(rule.visible_for(&[0.0]));
    assert!(!rule.visible_for(&[1.0]));
}
