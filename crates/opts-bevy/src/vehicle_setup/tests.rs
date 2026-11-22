//! Tests of the start procedures on small stand-in scripts (the real vehicles' procedures are
//! checked by `crates/opts-game/tests/vehicle_start.rs`).

mod battery_off;
mod degrees;
mod families;
mod mouse;
mod presses;
mod stepping;

use super::*;

/// A MAN-like vehicle: the battery key puts the busbar up, the start key (held) starts.
const MAN_LIKE: &str = "\
{init}
    120 (S.L.engine_tank_content)
{end}
{trigger:cp_batterietrennschalter_toggle}
    1 (S.L.elec_busbar_main)
{end}
{trigger:kw_m_enginestart}
    (L.L.elec_busbar_main) 0.3 >
    {if}
        1 (S.L.engine_starter)
    {endif}
{end}
{trigger:kw_m_enginestart_off}
    (L.L.engine_starter)
    {if}
        1 (S.L.engine_on)
    {endif}
    0 (S.L.engine_starter)
{end}
{trigger:kw_scheinwerfer_toggle}
    1 (S.L.lights_abbl)
{end}
";

/// A stepped-key vehicle: owns `cp_keyboard_e_m`, which counts the battery key's presses.
const STEPPED: &str = "\
{trigger:cp_batterietrennschalter_toggle}
    (L.L.cp_keyboard_e_m) 1 + (S.L.cp_keyboard_e_m)
    (L.L.cp_keyboard_e_m) 1 =
    {if}
        1 (S.L.elec_busbar_main)
    {endif}
    (L.L.cp_keyboard_e_m) 3 =
    {if}
        1 (S.L.engine_on)
    {endif}
{end}
";

fn engine(src: &str) -> ScriptEngine {
    let (mut e, diags) = ScriptEngine::from_sources(&[("t.osc", src)]);
    assert!(diags.is_empty(), "{diags:?}");
    e.run_init();
    e
}

/// Runs the recipe for `seconds` of 60 Hz frames.
fn drive(e: &mut ScriptEngine, run: &mut SetupRun, setup: &VehicleSetup, seconds: f32) -> bool {
    let mut settled = false;
    let mut now = 0.0;
    while now < seconds {
        now += 1.0 / 60.0;
        e.run_frame(1.0 / 60.0, false);
        settled = run.advance(e, setup, now);
    }
    settled
}

fn at(degree: f32) -> VehicleSetup {
    VehicleSetup {
        degree,
        ..VehicleSetup::default()
    }
}
