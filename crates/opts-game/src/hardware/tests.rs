//! Tests of the hardware integration: state machine, page lines, messages, control document.

use std::time::{Duration, Instant};

use opts_gui::l10n::Localization;
use opts_hardware::{Fact, HardwareProfile};

use super::*;
use crate::settings::GameSettings;
use crate::settings_registry::{SettingsRegistry, SettingKind};

fn loc() -> Localization {
    let mut l = Localization::new("en");
    crate::settings_registry::add_messages(&mut l);
    l
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("opts-game-hw-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn wait(state: &mut HardwareState) {
    let end = Instant::now() + Duration::from_secs(10);
    while state.poll().is_none() {
        assert!(Instant::now() < end, "probe did not finish");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn switch_defaults_off_and_actions_exist_in_the_advanced_category() {
    let r = SettingsRegistry::with_builtin();
    let allow = r.get(ALLOW_REPORTS_KEY).expect("registered");
    assert_eq!(allow.default, "false");
    assert!(matches!(allow.kind, SettingKind::Bool));
    for k in ["advanced.hardware_redetect", "advanced.hardware_delete"] {
        let d = r.get(k).expect(k);
        assert!(matches!(d.kind, SettingKind::Action));
        assert_eq!(d.category, crate::settings_registry::Category::Advanced);
    }
}

#[test]
fn probe_runs_off_thread_stores_only_when_asked_and_delete_removes_the_file() {
    let dir = scratch("state");
    let mut s = HardwareState::with_dir(Some(dir.clone()));
    s.start_probe(false, false);
    assert!(s.is_probing());
    wait(&mut s);
    assert!(s.profile.is_some());
    assert!(!s.is_stored(), "nothing is written unless the user asked");

    s.start_probe(true, true);
    let waiting = loop {
        if let Some(n) = s.poll() {
            break n;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(waiting, "the user who clicked Re-detect is told");
    assert!(s.is_stored());
    assert!(s.delete_stored());
    assert!(!s.is_stored());
    assert!(!s.delete_stored());
    assert!(s.profile.is_some(), "the profile in memory stays");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn page_lines_have_both_languages_for_known_and_unknown_profiles() {
    let l = loc();
    let mut known = HardwareProfile::default();
    known.cpu.logical_cores = Fact::Known(8);
    for p in [HardwareProfile::default(), known, opts_hardware::probe()] {
        for (key, vars) in view::lines(&p, &l) {
            assert!(l.has("en", key) && l.has("de", key), "{key}");
            assert!(vars.iter().all(|(_, v)| !v.contains('⟦')), "{key}: unresolved message");
        }
    }
    for r in opts_hardware::Reason::ALL {
        let k = format!("hw-reason-{}", r.key());
        assert!(l.has("en", &k) && l.has("de", &k), "{k}");
    }
    for k in ["yes", "unknown", "no-not_nvidia", "no-os_unsupported", "no-backend_unsupported", "no-no_ray_tracing", "no-no_runtime"] {
        let k = format!("hw-dlss-{k}");
        assert!(l.has("en", &k) && l.has("de", &k), "{k}");
    }
    for k in [
        "settings-info-hw-title", "settings-info-hw-privacy", "settings-info-hw-detecting",
        "settings-info-hw-stored", "settings-info-hw-not-stored", "settings-toast-hw-detecting",
        "settings-toast-hw-deleted", "settings-toast-hw-nothing-stored",
    ] {
        assert!(l.has("en", k) && l.has("de", k), "{k}");
    }
}

#[test]
fn control_document_reports_state_recommendations_and_the_report_switch() {
    let mut world = World::new();
    let mut s = HardwareState::with_dir(None);
    s.start_probe(false, false);
    wait(&mut s);
    world.insert_resource(s);
    world.insert_resource(SettingsRegistry::with_builtin());
    world.insert_resource(GameSettings::default());
    let doc = protocol::document(&world);
    assert_eq!(doc["probing"], false);
    assert_eq!(doc["stored"], false);
    assert_eq!(doc["allow_in_reports"], false);
    assert_eq!(doc["profile"]["schema"], 1);
    assert!(doc["recommendations"]["tile_radius"].is_string());
    assert!(doc["profile"]["cpu"]["logical_cores"].is_object());
}
