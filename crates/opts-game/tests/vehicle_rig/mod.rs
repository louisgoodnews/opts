//! The headless vehicle rig shared by the start tests: a vehicle's own scripts with the host
//! callbacks of a headless run, driven by frames, clicks and key presses (M-0141).

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
