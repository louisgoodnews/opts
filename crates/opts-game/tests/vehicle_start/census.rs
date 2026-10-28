//! Sound census and cold-start setup tests for reference vehicles.

use super::*;

#[test]
#[ignore = "census of the engine sounds of a vehicle's sound.cfg; prints"]
fn census_engine_sounds() {
    use opts_game::sound_logic::{playback_of, Playback};
    let Some(root) = std::env::var_os("OMSI_PATH").map(std::path::PathBuf::from) else {
        return;
    };
    let install = opts_bevy::OmsiInstall::new(root);
    for (label, rel) in [
        (
            "Citybus 530",
            "Vehicles/Citybus 530 by Kajosoft/01a_o530_e2_2.bus",
        ),
        ("MAN NL", "Vehicles/MAN_NL_NG/MAN_EN92_main.bus"),
        (
            "Bremen NG313",
            "Vehicles/HB76_MAN_LionsCity/MAN_NG313_main.bus",
        ),
    ] {
        let Some(bus_path) = install.find_file(rel) else {
            continue;
        };
        let bus_dir = bus_path.parent().unwrap().to_path_buf();
        let manifest = BusManifest::parse_bytes(&std::fs::read(&bus_path).unwrap()).unwrap();
        let Some(cfg) = manifest
            .sound_cfg
            .as_ref()
            .and_then(|r| install.find_in(&bus_dir, r))
        else {
            println!("SND {label}: no sound.cfg");
            continue;
        };
        let text: String = std::fs::read(&cfg)
            .unwrap()
            .iter()
            .map(|&b| b as char)
            .collect();
        let config = opts_core::formats::sound::SoundConfig::parse(&text);
        let e = load(rel).unwrap();
        println!(
            "SND {label}: {} definitions in {}",
            config.sounds.len(),
            cfg.display()
        );
        let mut counts = std::collections::BTreeMap::<&str, usize>::new();
        for d in &config.sounds {
            let vars: Vec<&str> = d
                .pitch_variable
                .iter()
                .map(String::as_str)
                .chain(d.conditions.iter().map(|c| c.variable.as_str()))
                .chain(d.volcurves.iter().map(|c| c.variable.as_str()))
                .collect();
            let engine_related = vars.iter().any(|v| {
                let v = v.to_lowercase();
                v.contains("engine")
                    || v.contains("antrieb")
                    || v.contains("gear")
                    || v.contains("throttle")
                    || v.contains("motor")
            });
            let playback = playback_of(d);
            let unknown: Vec<&&str> = vars
                .iter()
                .filter(|v| !e.writes(v) && !matches!(**v, "velocity" | "throttle" | "brake"))
                .collect();
            *counts
                .entry(match playback {
                    Playback::Loop => "loop",
                    Playback::OnEdge => "edge",
                    Playback::OnTrigger => "trigger",
                    Playback::Unsupported => "unsupported",
                })
                .or_default() += 1;
            let warn = d.file.to_lowercase().contains("warn");
            if engine_related || warn {
                println!(
                    "SND   {} | {:?} | pitch {:?} ref {} x{} | cond {:?} | vol {:?} | {:?} | vars not written by scripts: {:?}",
                    d.file, d.kind, d.pitch_variable, d.pitch_reference, d.pitch_scale,
                    d.conditions.iter().map(|c| format!("{} m{} {}", c.variable, c.mode, c.value)).collect::<Vec<_>>(),
                    d.volcurves.iter().map(|c| c.variable.clone()).collect::<Vec<_>>(),
                    playback, unknown
                );
            }
        }
        println!("SND {label}: by kind {counts:?}");
    }
}

#[test]
fn cold_start_buses_start_through_the_setup() {
    for (rel, recipe) in [
        ("Vehicles/Luzern_Bus/VBL_Citybus_main.bus", "three-step key"),
        (
            "Vehicles/Valiant_Citybus_Family/V_7000_2.bus",
            "three-step key (battery button)",
        ),
        (
            "Vehicles/MB_C2_EN_BVG/MB_C2_E6_Solo_BVG.bus",
            "ignition key turned by mouse",
        ),
        // The start key steps (ignition, then starter); the starter is held until the engine
        // runs on its own.
        (
            "Vehicles/TH_Ueberlandbus/O550_Euro3.bus",
            "battery switch and start key",
        ),
        (
            "Vehicles/TH_Ueberlandbus/O550_Euro3_Automatik.bus",
            "battery switch and start key",
        ),
        // The second starter press waits for the ABS self-test.
        (
            "Vehicles/[SP] Studio Polygon Lite/Lite_11.5m_D.bus",
            "three-step key (master switch)",
        ),
        ("Vehicles/Citybus_S31x/S315_1+2_E3.bus", "four-step key"),
    ] {
        let Some((on, n, name)) = start_with_setup(rel) else {
            eprintln!("OMSI_PATH / {rel} not available; skipping");
            continue;
        };
        assert_eq!(name, recipe, "{rel}");
        assert_eq!(on, 1.0, "{rel}: engine running");
        assert!(n > 400.0, "{rel}: idle speed {n}");
    }
}
