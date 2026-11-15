use super::*;

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
