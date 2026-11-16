//! A start whose key is turned with the mouse: the electrics, the drag past the starter.

use super::*;

/// A key turned with the mouse, in a part that also has the `engine_start_legacy` constant:
/// electrics from 0.2, the starter past 0.9, springs back to a third on release.
pub(super) const MOUSE_KEY: &str = "\
{trigger:cp_key_rot_drag}
    (L.S.mouse_y) -30 / (L.L.cp_key_rot) + 0 max 1 min (S.L.cp_key_rot) s1
    l1 0.2 >
    {if}
        1 (S.L.elec_busbar_main)
    {endif}
    l1 0.9 > (L.L.elec_busbar_main) &&
    {if}
        1 (S.L.engine_on)
    {endif}
{end}
{trigger:cp_key_rot_off}
    (L.L.cp_key_rot) 0.33 min (S.L.cp_key_rot)
{end}
";

fn mouse_key() -> ScriptEngine {
    let mut e = engine(MOUSE_KEY);
    e.consts_mut().set_scalar("engine_start_legacy", 0.0);
    e
}

#[test]
fn a_mouse_turned_key_stops_at_the_electrics_and_springs_back() {
    let mut e = mouse_key();
    assert_eq!(recipe_for(&e).name, "ignition key turned by mouse");
    drive(&mut e, &mut SetupRun::default(), &at(ELECTRICS), 4.0);
    assert_eq!(e.get("elec_busbar_main"), 1.0, "electrics on");
    assert_eq!(
        e.get("engine_on"),
        0.0,
        "not started at the electrics degree"
    );
    assert!(
        e.get("cp_key_rot") < 0.4,
        "key sprang back: {}",
        e.get("cp_key_rot")
    );
}

#[test]
fn a_mouse_turned_key_is_dragged_past_the_starter_position() {
    let mut e = mouse_key();
    drive(&mut e, &mut SetupRun::default(), &at(ENGINE), 8.0);
    assert_eq!(e.get("engine_on"), 1.0, "started past 0.9");
}
