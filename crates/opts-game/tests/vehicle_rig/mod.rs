//! The headless vehicle rig shared by the start tests: a vehicle's own scripts with the host
//! callbacks of a headless run, driven by frames, clicks and key presses (M-0141).

#![allow(dead_code)]

use opts_core::formats::vehicles::bus::BusManifest;
use opts_sim::script::ScriptEngine;

/// The scripts of a vehicle, initialised, with the host callbacks of a headless run.
pub fn load(rel: &str) -> Option<ScriptEngine> {
    let root = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from)?;
    let install = opts_bevy::OmsiInstall::new(root);
    let bus_path = install.find_file(rel)?;
    let bus_dir = bus_path.parent()?.to_path_buf();
    let manifest = BusManifest::parse_bytes(&std::fs::read(&bus_path).ok()?).ok()?;
    let (mut engine, _) =
        opts_sim::script::load_vehicle_scripts(&manifest, |r| install.find_in(&bus_dir, r));
    let ctx = opts_bevy::host_callbacks::VehicleHostContext {
        seed: 42,
        route: opts_bevy::route::RouteHandle::new(None),
        script_textures: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        fonts: std::sync::Arc::new(std::sync::RwLock::new(opts_bevy::text_texture::load_fonts(
            &install,
        ))),
        texture_base_dir: Some(bus_dir.join("Texture")),
        ..Default::default()
    };
    engine.set_external_handler(opts_bevy::host_callbacks::vehicle_callbacks_with_context(
        ctx,
    ));
    engine.run_init();
    Some(engine)
}

pub const DT: f32 = 1.0 / 60.0;

/// Runs `seconds` of simulation.
pub fn run(engine: &mut ScriptEngine, seconds: f32) {
    for _ in 0..(seconds / DT) as u32 {
        engine.run_frame(DT, false);
    }
}

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
