//! Start procedures of the reference vehicles (M-0141): the vehicles' own scripts, driven the way
//! a player drives them (hot-spot press/drag/release, keys), from a cold start to a running engine.
//! The procedures are written down in `docs/src/quality/vehicle-start-procedures.md`.
//!
//! Needs an OMSI installation (`OMSI_PATH`); skips without one.

use opts_core::formats::vehicles::bus::BusManifest;
use opts_sim::script::ScriptEngine;

/// The scripts of a vehicle, initialised, with the host callbacks of a headless run.
fn load(rel: &str) -> Option<ScriptEngine> {
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
    };
    engine.set_external_handler(opts_bevy::host_callbacks::vehicle_callbacks_with_context(
        ctx,
    ));
    engine.run_init();
    Some(engine)
}

const DT: f32 = 1.0 / 60.0;

/// Runs `seconds` of simulation.
fn run(engine: &mut ScriptEngine, seconds: f32) {
    for _ in 0..(seconds / DT) as u32 {
        engine.run_frame(DT, false);
    }
}

/// A click like `vehicle.click`: press, `frames` frames of drag carrying `drag_y` pixels in all,
/// hold for `hold_s`, release.
fn click(engine: &mut ScriptEngine, name: &str, drag_y: f32, frames: u32, hold_s: f32) {
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

#[test]
fn citybus_530_starts_through_its_ignition_key() {
    let Some(mut e) = load("Vehicles/Citybus 530 by Kajosoft/01a_o530_e2_2.bus") else {
        eprintln!("OMSI_PATH / Citybus 530 not available; skipping");
        return;
    };
    run(&mut e, 1.0);
    assert_eq!(e.get("engine_on"), 0.0, "the bus starts cold");
    assert_eq!(e.get("elec_busbar_main"), 0.0, "with the electrics off");

    // Key to the drive position, released (the first release clears the starter lock-out).
    click(&mut e, "cp_kluczyk_mov", -50.0, 60, 0.2);
    run(&mut e, 3.0);
    assert!(
        e.get("elec_busbar_main") > 0.3,
        "electrics on: {}",
        e.get("elec_busbar_main")
    );
    assert_eq!(e.get("engine_on"), 0.0);

    // Key to the starter, held until the engine runs.
    click(&mut e, "cp_kluczyk_mov", -20.0, 30, 4.0);
    assert_eq!(e.get("engine_on"), 1.0, "engine running");
    run(&mut e, 2.0);
    let n = e.get("engine_n");
    assert!((400.0..900.0).contains(&n), "idle rpm {n}");
}

/// A key press and release (`<trigger>`, `<trigger>_off`) with `hold_s` of simulation in between.
fn tap(engine: &mut ScriptEngine, trigger: &str, hold_s: f32) {
    engine.trigger(trigger);
    run(engine, hold_s);
    engine.trigger(&format!("{trigger}_off"));
}

#[test]
fn citybus_530_starts_and_stops_with_the_keyboard_steps() {
    let Some(mut e) = load("Vehicles/Citybus 530 by Kajosoft/01a_o530_e2_2.bus") else {
        return;
    };
    run(&mut e, 1.0);
    assert_eq!(e.get("engine_on"), 0.0);

    // `battery` (E) steps the ignition key: electrics, drive position, starter (held).
    tap(&mut e, "cp_batterietrennschalter_toggle", 0.3);
    run(&mut e, 2.0);
    assert_eq!(e.get("cp_kluczyk_rot_mode"), 1.0);
    assert!(e.get("elec_busbar_main") > 0.3);
    tap(&mut e, "cp_batterietrennschalter_toggle", 0.3);
    run(&mut e, 2.0);
    assert_eq!(e.get("cp_kluczyk_rot_mode"), 2.0);
    assert_eq!(e.get("engine_on"), 0.0);
    tap(&mut e, "cp_batterietrennschalter_toggle", 3.0);
    run(&mut e, 2.0);
    assert_eq!(e.get("engine_on"), 1.0, "engine running");
    assert_eq!(
        e.get("cp_kluczyk_rot_mode"),
        2.0,
        "the key sprang back to drive"
    );

    // `engine_start` (M) steps it back: engine off, then electrics off.
    tap(&mut e, "kw_m_enginestart", 0.3);
    run(&mut e, 3.0);
    assert_eq!(e.get("engine_on"), 0.0, "engine stopped");
    assert!(e.get("elec_busbar_main") > 0.3, "the electrics stay on");
    tap(&mut e, "kw_m_enginestart", 0.3);
    run(&mut e, 3.0);
    assert!(e.get("elec_busbar_main") < 0.3, "electrics off");
}

#[test]
fn man_nl_starts_with_the_battery_key_and_the_start_key() {
    let Some(mut e) = load("Vehicles/MAN_NL_NG/MAN_EN92_main.bus") else {
        eprintln!("OMSI_PATH / MAN NL not available; skipping");
        return;
    };
    run(&mut e, 1.0);
    assert_eq!(e.get("engine_on"), 0.0, "cold");

    // Battery switch, then the start key held until the engine runs.
    tap(&mut e, "cp_batterietrennschalter_toggle", 0.3);
    run(&mut e, 2.0);
    assert!(e.get("elec_busbar_main") > 0.3, "electrics on");
    tap(&mut e, "kw_m_enginestart", 3.5);
    run(&mut e, 4.0);
    assert_eq!(e.get("engine_on"), 1.0, "engine running");
    let n = e.get("engine_n");
    assert!((400.0..700.0).contains(&n), "idle rpm {n}");

    // The start key on a running engine shuts it down (the actuator needs air).
    run(&mut e, 5.0);
    tap(&mut e, "kw_m_enginestart", 1.5);
    run(&mut e, 6.0);
    assert_eq!(e.get("engine_on"), 0.0, "engine stopped");
}

#[test]
fn c2_starts_with_three_key_steps_and_the_start_key() {
    let Some(mut e) = load("Vehicles/BRT_C2-Solo/C2_2T.bus") else {
        eprintln!("OMSI_PATH / BRT C2 not available; skipping");
        return;
    };
    run(&mut e, 1.0);
    assert_eq!(e.get("engine_on"), 0.0);

    // `battery` (E) inserts the key, turns it to the electrics, then to the drive position.
    tap(&mut e, "cp_batterietrennschalter_toggle", 0.3);
    run(&mut e, 2.0);
    assert!(e.get("elec_busbar_main") < 0.3, "inserted, not yet turned");
    tap(&mut e, "cp_batterietrennschalter_toggle", 0.3);
    run(&mut e, 2.5);
    assert!(e.get("elec_busbar_main") > 0.3, "electrics on");
    tap(&mut e, "cp_batterietrennschalter_toggle", 0.3);
    run(&mut e, 2.0);
    assert_eq!(e.get("engine_on"), 0.0);

    // The start key from the drive position, held until the engine runs.
    tap(&mut e, "kw_m_enginestart", 3.5);
    run(&mut e, 4.0);
    assert_eq!(e.get("engine_on"), 1.0, "engine running");
}

#[test]
fn sd200_and_ng272_start_like_the_man_nl() {
    for rel in [
        "Vehicles/MAN_SD200/MAN_SD77.bus",
        "Vehicles/MAN_NL_NG/MAN_GN92_main.bus",
    ] {
        let Some(mut e) = load(rel) else {
            eprintln!("{rel} not available; skipping");
            continue;
        };
        run(&mut e, 1.0);
        assert_eq!(e.get("engine_on"), 0.0, "{rel} cold");
        tap(&mut e, "cp_batterietrennschalter_toggle", 0.3);
        run(&mut e, 2.0);
        assert!(e.get("elec_busbar_main") > 0.3, "{rel} electrics on");
        tap(&mut e, "kw_m_enginestart", 3.5);
        run(&mut e, 4.0);
        assert_eq!(e.get("engine_on"), 1.0, "{rel} engine running");
    }
}
