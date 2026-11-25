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
        last_target: Vec::new(),
    };
    assert!(rule.visible_for(&[0.0]));
    assert!(!rule.visible_for(&[1.0]));
}

#[test]
fn snow_visible_is_one_only_while_the_session_has_snow() {
    use super::super::clock::SceneryClock;
    let var = VarRef::Host(HostVar::of("snow_visible"));
    let mut clock = SceneryClock::at(2026, 1, 15, 12.0 * 3600.0);
    assert_eq!(read_var(var, None, &HostValues::of(&clock), 0.0), 0.0);
    clock.weather.precip_type = 2.0;
    assert_eq!(read_var(var, None, &HostValues::of(&clock), 0.0), 1.0);
    clock.weather.precip_type = 1.0;
    assert_eq!(read_var(var, None, &HostValues::of(&clock), 0.0), 0.0);
}

#[test]
fn the_half_year_variables_follow_the_month() {
    use super::super::clock::SceneryClock;
    let winter = VarRef::Host(HostVar::of("winterhalbjahr"));
    let summer = VarRef::Host(HostVar::of("sommerhalbjahr"));
    let january = HostValues::of(&SceneryClock::at(2026, 1, 15, 12.0 * 3600.0));
    assert_eq!(read_var(winter, None, &january, 0.0), 1.0);
    assert_eq!(read_var(summer, None, &january, 0.0), 0.0);
    let july = HostValues::of(&SceneryClock::at(2026, 7, 15, 12.0 * 3600.0));
    assert_eq!(read_var(winter, None, &july, 0.0), 0.0);
    assert_eq!(read_var(summer, None, &july, 0.0), 1.0);
}
