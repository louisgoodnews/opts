//! The battery switch's off press (ISS-014): pressing the toggle and releasing it switches the
//! busbar off, so the electrical outputs read zero. The stand-in is written for this test in the
//! shape of the game's switch (no game script is copied): the toggle flips the switch, its off
//! branch clears it, and the frame rule makes the busbar follow the switch.

use super::*;

/// The switch flips on each press. The `{else}` branch is the off press; a press releases with
/// `<trigger>_off`, as a held key does.
const SWITCH: &str = "\
{init}
    0 (S.L.elec_busbar_main_sw)
{end}
{trigger:cp_batterietrennschalter_toggle}
    (L.L.elec_busbar_main_sw) !
    {if}
        1 (S.L.elec_busbar_main_sw)
    {else}
        0 (S.L.elec_busbar_main_sw)
    {endif}
{end}
{trigger:cp_batterietrennschalter_toggle_off}
    0 (S.L.cp_batterietrennschalter_antirepeat)
{end}
{frame}
    (L.L.elec_busbar_main_sw) (S.L.elec_busbar_main)
{end}
";

/// A key press and release with `secs` of frames between them, as the rig's `tap` does.
fn press(e: &mut ScriptEngine, trigger: &str, secs: f32) {
    e.trigger(trigger);
    for _ in 0..(secs * 60.0) as u32 {
        e.run_frame(1.0 / 60.0, false);
    }
    e.trigger(&format!("{trigger}_off"));
    e.run_frame(1.0 / 60.0, false);
}

#[test]
fn releasing_the_battery_switch_off_takes_the_busbar_to_zero() {
    let mut e = engine(SWITCH);
    press(&mut e, "cp_batterietrennschalter_toggle", 0.3);
    assert_eq!(e.get("elec_busbar_main_sw"), 1.0, "switch on");
    assert_eq!(e.get("elec_busbar_main"), 1.0, "busbar on");

    press(&mut e, "cp_batterietrennschalter_toggle", 0.3);
    assert_eq!(e.get("elec_busbar_main_sw"), 0.0, "switch off");
    assert_eq!(
        e.get("elec_busbar_main"),
        0.0,
        "busbar follows the switch off"
    );
}
