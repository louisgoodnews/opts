//! Start procedures of the reference vehicles (M-0141): the vehicles' own scripts, driven the way
//! a player drives them (hot-spot press/drag/release, keys), from a cold start to a running engine.
//! The procedures are written down in `docs/src/quality/vehicle-start-procedures.md`.
//!
//! Needs an OMSI installation (`OMSI_PATH`); skips without one.

mod vehicle_rig;

use opts_core::formats::vehicles::bus::BusManifest;
use vehicle_rig::{click, load, run, start_with_setup, tap};

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

#[test]
#[ignore = "census; prints"]
fn census_tram() {
    for rel in [
        "Vehicles/Tram_NF6D/NF6D_B_User.bus",
        "Vehicles/Tram_NF6D/NF6D_A.bus",
    ] {
        let Some(mut e) = load(rel) else {
            eprintln!("CENSUS {rel}: not available");
            continue;
        };
        run(&mut e, 1.0);
        let names = [
            "cp_batterietrennschalter_toggle",
            "kw_m_enginestart",
            "kw_s_1",
            "kw_s_2",
            "kw_s_R",
            "kw_s_N",
            "kw_s_plus",
            "kw_scheinwerfer_toggle",
            "bus_doorfront0",
            "horn",
            "parking_brake_toggle",
        ];
        let has: Vec<_> = names.iter().filter(|n| e.has_trigger(n)).collect();
        let before = e.variables();
        tap(&mut e, "cp_batterietrennschalter_toggle", 0.3);
        run(&mut e, 2.0);
        let after = e.variables();
        let changed: Vec<_> = after
            .iter()
            .filter(|(k, v)| {
                before
                    .iter()
                    .any(|(k2, v2)| k2 == k && (v2 - v).abs() > 1e-4)
            })
            .map(|(k, v)| format!("{k}={v:.2}"))
            .take(25)
            .collect();
        eprintln!("CENSUS {rel}: triggers {has:?}\nCENSUS battery key changed: {changed:?}");
    }
}

/// The `[mouseevent]` names of a model configuration (a line `[mouseevent]` and the name after it).
fn mouse_events_of(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = text.lines().map(str::trim);
    while let Some(line) = lines.next() {
        if line.eq_ignore_ascii_case("[mouseevent]") {
            if let Some(name) = lines.by_ref().find(|l| !l.is_empty()) {
                out.push(name.to_owned());
            }
        }
    }
    out.sort_unstable_by_key(|n| n.to_lowercase());
    out.dedup();
    out
}

#[test]
#[ignore = "census for docs/src/quality/controls-report.md; prints markdown"]
fn census_controls_report() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
        return;
    };
    let install = opts_bevy::OmsiInstall::new(root);
    let keyboard = install
        .find_file("Inputs/keyboard.cfg")
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| opts_core::formats::engine_config::KeyboardConfig::parse(&b))
        .unwrap_or_default();
    let key_triggers: Vec<String> = {
        let mut v: Vec<String> = keyboard
            .bindings
            .iter()
            .filter(|b| b.section == opts_core::formats::engine_config::KeySection::Vehicles)
            .map(|b| b.action.clone())
            .collect();
        v.sort();
        v.dedup();
        v
    };
    for (label, rel) in [
        (
            "Citybus 530",
            "Vehicles/Citybus 530 by Kajosoft/01a_o530_e2_2.bus",
        ),
        ("MAN NL", "Vehicles/MAN_NL_NG/MAN_EN92_main.bus"),
        ("MAN SD200", "Vehicles/MAN_SD200/MAN_SD77.bus"),
        ("MAN NG272", "Vehicles/MAN_NL_NG/MAN_GN92_main.bus"),
        ("BRT C2", "Vehicles/BRT_C2-Solo/C2_2T.bus"),
        ("Tram NF6D", "Vehicles/Tram_NF6D/NF6D_B_User.bus"),
    ] {
        let Some(bus_path) = install.find_file(rel) else {
            println!("REPORT | {label} | not installed");
            continue;
        };
        let bus_dir = bus_path.parent().unwrap().to_path_buf();
        let manifest = BusManifest::parse_bytes(&std::fs::read(&bus_path).unwrap()).unwrap();
        let model = manifest
            .model_cfg
            .as_ref()
            .and_then(|r| install.find_in(&bus_dir, r))
            .and_then(|p| std::fs::read(p).ok())
            .map(|b| b.iter().map(|&c| c as char).collect::<String>())
            .unwrap_or_default();
        let spots = mouse_events_of(&model);
        let Some(e) = load(rel) else { continue };
        let handled = |n: &str| {
            e.has_trigger(n)
                || e.has_trigger(&format!("{n}_drag"))
                || e.has_trigger(&format!("{n}_off"))
        };
        let dead: Vec<&String> = spots.iter().filter(|n| !handled(n)).collect();
        let keys_dead: Vec<&String> = key_triggers.iter().filter(|t| !handled(t)).collect();
        println!(
            "REPORT | {label} | {} hot-spots, {} unhandled | {} keyboard.cfg vehicle triggers, {} unhandled",
            spots.len(),
            dead.len(),
            key_triggers.len(),
            keys_dead.len()
        );
        println!(
            "REPORT-SPOTS {label}: {}",
            dead.iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "REPORT-KEYS {label}: {}",
            keys_dead
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

#[path = "vehicle_start/census.rs"]
mod census;
