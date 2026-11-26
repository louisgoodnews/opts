//! The start tests' input on the vehicle rig: a click on a hot spot, a key press and the
//! game's own set-up. Used by the start procedures only (`vehicle_start`).

use opts_sim::script::ScriptEngine;

use super::vehicle_rig::{load, run, DT};

/// A click like `vehicle.click`: press, `frames` frames of drag carrying `drag_y` pixels in all,
/// hold for `hold_s`, release.
pub fn click(engine: &mut ScriptEngine, name: &str, drag_y: f32, frames: u32, hold_s: f32) {
    engine.trigger(name);
    for _ in 0..frames {
        engine.set_system("mouse_x", 0.0);
        engine.set_system("mouse_y", drag_y / frames as f32);
        engine.trigger(&format!("{name}_drag"));
        engine.run_frame(DT, false);
    }
    engine.set_system("mouse_y", 0.0);
    run(engine, hold_s);
    engine.trigger(&format!("{name}_off"));
}

/// A key press and release (`<trigger>`, `<trigger>_off`) with `hold_s` of simulation in between.
pub fn tap(engine: &mut ScriptEngine, trigger: &str, hold_s: f32) {
    engine.trigger(trigger);
    run(engine, hold_s);
    engine.trigger(&format!("{trigger}_off"));
}

/// Runs the game's own set-up (`SetupRun`, default degree: engine running) on a vehicle; returns
/// the engine flag and speed two seconds after it settled, or `None` without an installation.
pub fn start_with_setup(rel: &str) -> Option<(f32, f32, &'static str)> {
    let mut e = load(rel)?;
    let setup = opts_bevy::vehicle_setup::VehicleSetup::default();
    let mut run_ = opts_bevy::vehicle_setup::SetupRun::default();
    let mut now = 0.0;
    while now < 30.0 {
        let done = run_.advance(&mut e, &setup, now);
        e.run_frame(DT, false);
        now += DT;
        if done && now > 5.0 && e.get("engine_on") > 0.5 {
            break;
        }
    }
    run(&mut e, 2.0);
    Some((
        e.get("engine_on"),
        e.get("engine_n"),
        opts_bevy::vehicle_setup::recipe_for(&e).name,
    ))
}
