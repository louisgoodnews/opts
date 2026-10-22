use super::*;

#[test]
fn the_lever_moves_while_held_and_stays_where_it_is_left() {
    let mut c = RailControls::default();
    c.move_lever(true, false, 1.0);
    assert!((c.lever - 0.5).abs() < 1e-6);
    c.move_lever(false, false, 5.0);
    assert!((c.lever - 0.5).abs() < 1e-6, "no key: it stays");
    c.move_lever(true, false, 10.0);
    assert_eq!(c.lever, 1.0, "clamped at full traction");
    c.move_lever(false, true, 10.0);
    assert_eq!(c.lever, -1.0, "and at full braking");
    c.move_lever(true, true, 10.0);
    assert_eq!(c.lever, -1.0, "both keys cancel");
}

#[test]
fn the_lever_is_the_throttle_above_zero_and_the_brake_below() {
    let mut c = RailControls {
        lever: 0.4,
        ..RailControls::default()
    };
    assert_eq!(c.pedals(), (0.4, 0.0));
    c.lever = -0.7;
    assert_eq!(c.pedals(), (0.0, 0.7));
    c.lever = 0.0;
    assert_eq!(c.pedals(), (0.0, 0.0));
}

#[test]
fn a_direction_key_has_the_scripts_switches_set_again_whatever_the_direction_was() {
    let mut c = RailControls::default();
    c.select(1);
    assert!(c.direction_dirty && c.direction == 1);
    c.direction_dirty = false;
    c.select(-1);
    assert!(c.direction_dirty && c.direction == -1);
}

fn cab() -> ScriptEngine {
    let src = "{trigger:automatic_D}\n1 (S.L.a_kopf_drive)\n{end}\n\
               {trigger:RFahrt_A}\n(L.L.Rfahrt_A) 0 =\n{if}\n1 (S.L.Rfahrt_A)\n{else}\n0 (S.L.Rfahrt_A)\n{endif}\n{end}\n";
    let (e, diags) = ScriptEngine::from_sources(&[("t.osc", src)]);
    assert!(diags.is_empty(), "{diags:?}");
    e
}

#[test]
fn the_trams_reverse_switch_is_pressed_only_when_it_is_not_where_it_should_be() {
    let mut e = cab();
    assert!(apply_direction(&mut e, true));
    assert_eq!(e.get("rfahrt_a"), 1.0);
    assert_eq!(e.get("a_kopf_drive"), 1.0, "the drive position comes first");
    // Already reversing: nothing to press for the switch.
    apply_direction(&mut e, true);
    assert_eq!(e.get("rfahrt_a"), 1.0);
    apply_direction(&mut e, false);
    assert_eq!(e.get("rfahrt_a"), 0.0);
}

#[test]
fn a_vehicle_without_a_direction_switch_is_left_alone() {
    let (mut e, _) = ScriptEngine::from_sources(&[("t.osc", "{frame}\n{end}")]);
    assert!(!apply_direction(&mut e, true));
}
