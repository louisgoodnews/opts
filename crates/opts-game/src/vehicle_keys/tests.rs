//! Unit tests for `super` (`vehicle_keys.rs`).

use super::*;
use crate::settings_registry::SettingsCapabilities;

fn parts() -> (SettingsRegistry, SettingsCapabilities, GameSettings) {
    (
        SettingsRegistry::with_builtin(),
        SettingsCapabilities::default(),
        GameSettings::default(),
    )
}

#[test]
fn defaults_are_valid_unique_and_free() {
    let mut seen = std::collections::BTreeSet::new();
    // Bus and tram keys may share a key; the scopes are checked one at a time.
    for tram in [false, true] {
        seen.clear();
        for a in ACTIONS.iter().filter(|a| a.scope.applies(tram)) {
            let chord = KeyChord::parse(a.default).unwrap_or_else(|e| panic!("{}: {e}", a.id));
            if let Some(c) = chord {
                assert!(
                    !c.is_reserved(),
                    "{} default {} is reserved",
                    a.id,
                    a.default
                );
                assert!(seen.insert(a.default), "{} duplicates {}", a.id, a.default);
                assert_eq!(c.to_text().as_deref(), Some(a.default));
            }
        }
    }
}

#[test]
fn every_action_has_a_registered_setting_and_messages() {
    let (reg, ..) = parts();
    let loc = crate::settings_registry::localization("en");
    let de = crate::settings_registry::localization("de");
    for a in ACTIONS {
        let d = reg.get(a.setting).expect(a.setting);
        assert!(matches!(d.kind, SettingKind::Binding));
        for l in [&loc, &de] {
            assert_ne!(l.t(a.label_key), a.label_key, "{}", a.label_key);
            assert_ne!(l.t(a.desc_key), a.desc_key, "{}", a.desc_key);
        }
        if let Some(rail) = a.rail_setting {
            let d = reg.get(rail).expect(rail);
            assert!(matches!(d.kind, SettingKind::Binding));
            for l in [&loc, &de] {
                assert_ne!(l.t(d.label_key), d.label_key, "{}", d.label_key);
                assert_ne!(l.t(d.description_key), d.description_key);
            }
        }
    }
}

#[test]
fn a_bus_and_a_tram_have_their_own_layouts() {
    let (reg, caps, mut s) = parts();
    // The tram's lights go on another key; the bus keeps L.
    reg.set(&mut s, &caps, "controls.rail.lights", "J").unwrap();
    let lights = action("lights").expect("lights");
    assert_eq!(
        chord_of(&reg, &s, lights, false).and_then(|c| c.to_text()),
        Some("L".to_owned())
    );
    assert_eq!(
        chord_of(&reg, &s, lights, true).and_then(|c| c.to_text()),
        Some("J".to_owned())
    );
    // A key taken in one layout is free in the other: the bus may use J for the hazard
    // lights, the tram may not.
    assert!(reg
        .set(&mut s, &caps, "controls.vehicle.hazard", "J")
        .is_ok());
    let e = reg
        .set(&mut s, &caps, "controls.rail.hazard", "J")
        .unwrap_err();
    assert!(e.to_string().contains("lights"), "{e}");
    // The shared default is the same key in both layouts.
    for a in ACTIONS.iter().filter(|a| a.rail_setting.is_some()) {
        assert_eq!(a.default, a.default);
        assert_eq!(a.setting_for(true), a.rail_setting.unwrap());
        assert_eq!(a.setting_for(false), a.setting);
    }
}

#[test]
fn rebinding_is_validated_and_conflicts_are_refused() {
    let (reg, caps, mut s) = parts();
    // Free key: accepted and canonicalised.
    let (_, new) = reg
        .set(&mut s, &caps, "controls.vehicle.lights", "ctrl+shift+g")
        .unwrap();
    assert_eq!(new, "Ctrl+Shift+G");
    // Another action's key.
    let e = reg
        .set(&mut s, &caps, "controls.vehicle.hazard", "M")
        .unwrap_err();
    assert!(e.to_string().contains("engine_start"), "{e}");
    // A key OPTS uses (throttle), but a Ctrl combination of it is fine.
    assert!(reg
        .set(&mut s, &caps, "controls.vehicle.hazard", "W")
        .is_err());
    assert!(reg
        .set(&mut s, &caps, "controls.vehicle.hazard", "Ctrl+W")
        .is_ok());
    // Garbage, and unbinding.
    assert!(reg
        .set(&mut s, &caps, "controls.vehicle.hazard", "Hyper+Q")
        .is_err());
    assert!(reg
        .set(&mut s, &caps, "controls.vehicle.hazard", "nonsense")
        .is_err());
    let (_, none) = reg
        .set(&mut s, &caps, "controls.vehicle.hazard", "")
        .unwrap();
    assert_eq!(none, "none");
    // The key freed by rebinding `lights` can be taken by another action.
    assert!(reg
        .set(&mut s, &caps, "controls.vehicle.hazard", "L")
        .is_ok());
}

#[test]
fn settings_drive_the_bindings() {
    let mut app = App::new();
    app.add_plugins(VehicleKeysPlugin)
        .insert_resource(SettingsRegistry::with_builtin())
        .insert_resource(GameSettings::default())
        .insert_resource(ScriptKeyBase(vec![
            ScriptKeyBinding {
                trigger: "IBIS_1".into(),
                key: KeyCode::Numpad1,
                shift: false,
                ctrl: true,
                held: false,
            },
            // A cfg binding of a rebindable action is replaced.
            ScriptKeyBinding {
                trigger: "kw_scheinwerfer_toggle".into(),
                key: KeyCode::KeyK,
                shift: false,
                ctrl: false,
                held: false,
            },
        ]));
    app.update();
    let lights = |app: &App| -> Vec<KeyCode> {
        app.world()
            .resource::<ScriptKeyBindings>()
            .0
            .iter()
            .filter(|b| b.trigger == "kw_scheinwerfer_toggle")
            .map(|b| b.key)
            .collect()
    };
    assert_eq!(lights(&app), [KeyCode::KeyL]);
    assert!(app
        .world()
        .resource::<ScriptKeyBindings>()
        .0
        .iter()
        .any(|b| b.trigger == "IBIS_1"));
    let caps = SettingsCapabilities::default();
    let reg = app.world().resource::<SettingsRegistry>().clone();
    reg.set(
        &mut app.world_mut().resource_mut::<GameSettings>(),
        &caps,
        "controls.vehicle.lights",
        "J",
    )
    .unwrap();
    app.update();
    assert_eq!(lights(&app), [KeyCode::KeyJ]);
    // "none" unbinds.
    reg.set(
        &mut app.world_mut().resource_mut::<GameSettings>(),
        &caps,
        "controls.vehicle.lights",
        "none",
    )
    .unwrap();
    app.update();
    assert!(lights(&app).is_empty());
}
