//! A stepping start key: pressed again after the interlock, and the running engine.

use super::*;

/// A start key that steps: the first press switches the ignition on, the second starts. The
/// engine fires at 300 rpm and runs on its own only from 380 rpm; letting go at the first firing
/// stalls it. An ABS self-test runs for 3 s after the ignition.
const STEPPING_START: &str = "\
{trigger:cp_batterietrennschalter_toggle}
    1 (S.L.elec_busbar_main)
{end}
{trigger:kw_m_enginestart}
    (L.L.ign) 0 =
    {if}
        1 (S.L.ign) (S.L.bremse_abs_selftest)
        0 (S.L.selftest_t)
    {else}
        (L.L.bremse_abs_selftest) !
        {if}
            1 (S.L.engine_starter)
        {endif}
    {endif}
{end}
{trigger:kw_m_enginestart_off}
    0 (S.L.engine_starter)
{end}
{frame}
    (L.L.selftest_t) (L.S.Timegap) + (S.L.selftest_t)
    (L.L.selftest_t) 3 > {if} 0 (S.L.bremse_abs_selftest) {endif}
    (L.L.engine_starter)
    {if}
        (L.L.engine_n) 450 (L.S.Timegap) * + 600 min (S.L.engine_n)
    {else}
        (L.L.engine_n) 380 <
        {if}
            0 (S.L.engine_n)
        {endif}
    {endif}
    (L.L.engine_n) 300 > (S.L.engine_on)
{end}
";

#[test]
fn a_stepping_start_key_is_pressed_again_after_the_interlock_and_held_until_the_engine_runs() {
    let mut e = engine(STEPPING_START);
    assert_eq!(recipe_for(&e).name, "battery switch and start key");
    drive(&mut e, &mut SetupRun::default(), &at(ENGINE), 30.0);
    assert_eq!(e.get("engine_on"), 1.0, "runs: n {}", e.get("engine_n"));
    assert!(
        e.get("engine_n") >= 380.0,
        "on its own: {}",
        e.get("engine_n")
    );
}

#[test]
fn a_running_engine_is_not_started_again() {
    let mut e = engine(MAN_LIKE);
    let mut run = SetupRun::default();
    drive(&mut e, &mut run, &at(ENGINE), 20.0);
    assert_eq!(e.get("engine_on"), 1.0);
    // The retry step was skipped: the start key's release would have stopped a second start.
    assert_eq!(e.get("engine_starter"), 0.0);
}

#[test]
fn all_of_several_conditions_must_hold() {
    let both =
        engine("{trigger:actia_move_drag}\n    1 (S.L.key_mode) (S.L.actia_dash_on)\n{end}\n");
    assert_eq!(recipe_for(&both).name, "four-step key");
    let without_trigger = engine("{init}\n    1 (S.L.key_mode) (S.L.actia_dash_on)\n{end}\n");
    assert_eq!(
        recipe_for(&without_trigger).name,
        "battery switch and start key"
    );
}
