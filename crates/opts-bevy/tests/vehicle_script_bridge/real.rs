//! The real install (skipped without `OMSI_PATH`): the keyboard configuration and the SD77 triggers.

use super::{BusManifest, KeyboardConfig};
use bevy::prelude::*;
use opts_bevy::ScriptKeyBindings;

/// Real installation (skipped unless `OMSI_PATH` is set): every `[vehicles]` entry of
/// `keyboard.cfg` has a known key, and the SD77's scripts handle the triggers that its keys
/// and `[mouseevent]` hot-spots raise.
#[test]
fn real_keyboard_cfg_and_sd77_triggers() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let install = opts_bevy::OmsiInstall::new(root.clone());
    let cfg_path = install
        .find_file("Inputs/keyboard.cfg")
        .expect("Inputs/keyboard.cfg");
    let cfg = KeyboardConfig::parse(&std::fs::read(cfg_path).unwrap());
    let vehicle: Vec<_> = cfg.vehicle_bindings().collect();
    assert!(vehicle.len() > 50, "only {} vehicle entries", vehicle.len());
    for b in &vehicle {
        assert!(
            opts_bevy::dik_to_keycode(b.scan_code).is_some(),
            "{}: unknown scan code {}",
            b.action,
            b.scan_code
        );
    }
    let bindings = ScriptKeyBindings::from_config(&cfg);
    eprintln!(
        "{} vehicle entries, {} bound",
        vehicle.len(),
        bindings.0.len()
    );

    let Some(bus_path) = install.find_file("Vehicles/MAN_SD200/MAN_SD77.bus") else {
        eprintln!("MAN_SD200 not installed; skipping the trigger check");
        return;
    };
    let bus_dir = bus_path.parent().unwrap().to_path_buf();
    let manifest = BusManifest::parse_bytes(&std::fs::read(&bus_path).unwrap()).unwrap();
    let (mut engine, _) = opts_sim::script::load_vehicle_scripts(&manifest, |rel| {
        super::read_script(install.find_in(&bus_dir, rel))
    });
    engine.run_init();
    for name in [
        "bus_doorfront0",
        "bus_dooraft",
        "kw_scheinwerfer_toggle",
        "horn",
    ] {
        assert!(engine.trigger(name), "SD77 has no trigger {name}");
    }
}
