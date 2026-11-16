//! The recipe is recognised from the scripts: a tram, a serial hybrid and the recognition rules.

use super::mouse::MOUSE_KEY;
use super::*;

/// A tram-like vehicle: scripts without an engine or a busbar.
const TRAM_LIKE: &str = "\
{trigger:kw_s_1}
    1 (S.L.traction)
{end}
";

#[test]
fn a_vehicle_without_an_engine_settles_at_once_and_counts_as_started() {
    let mut e = engine(TRAM_LIKE);
    let mut run = SetupRun::default();
    assert!(
        drive(&mut e, &mut run, &at(1.0), 6.0),
        "nothing to start, nothing to wait for"
    );
    assert!(!has_engine(&e));
    assert!(has_engine(&engine(MAN_LIKE)));
}

#[test]
fn recipes_recognise_a_trigger_and_a_constant() {
    let valiant = engine("{trigger:cp_start_key_rot_drag}\n    1 (S.L.x)\n{end}\n");
    assert_eq!(recipe_for(&valiant).name, "three-step key (battery button)");
    let mut legacy = engine(MOUSE_KEY);
    legacy.consts_mut().set_scalar("engine_start_legacy", 1.0);
    assert_eq!(
        recipe_for(&legacy).name,
        "battery switch and start key",
        "the legacy C2 takes the battery switch and start key"
    );
}

/// A serial hybrid (MB O530 GDH): the start key toggles drive readiness; the diesel generator
/// starts only on demand, so it stays off at standstill.
const SERIAL_HYBRID: &str = "\
{trigger:cp_batterietrennschalter_toggle}
    1 (S.L.elec_busbar_main)
{end}
{trigger:kw_m_enginestart}
    (L.L.startup_mode) !
    {if}
        (L.L.elec_busbar_main) 0.3 >
        {if}
            1 (S.L.startup_mode)
        {endif}
    {else}
        0 (S.L.startup_mode) (S.L.generator)
    {endif}
{end}
{macro:never}
    0 (S.L.generator_trigger)
{end}
";

#[test]
fn a_serial_hybrid_is_ready_with_the_diesel_off_and_its_start_key_is_pressed_once() {
    let mut e = engine(SERIAL_HYBRID);
    assert_eq!(recipe_for(&e).name, "serial hybrid start button");
    let mut run = SetupRun::default();
    drive(&mut e, &mut run, &at(0.75), 20.0);
    assert_eq!(
        e.get("startup_mode"),
        1.0,
        "a second press would switch it off again"
    );
    assert_eq!(e.get("engine_on"), 0.0);
    assert!(ready(&e));
    // An ordinary vehicle is ready only with its engine running.
    let man = engine(MAN_LIKE);
    assert!(!ready(&man));
}
