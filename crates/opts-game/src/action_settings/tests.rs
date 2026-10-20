use super::*;
use crate::settings_registry::{SettingsCapabilities, SettingsRegistry};

fn parts() -> (SettingsRegistry, SettingsCapabilities, GameSettings) {
    (
        SettingsRegistry::with_builtin(),
        SettingsCapabilities::default(),
        GameSettings::default(),
    )
}

#[test]
fn defaults_are_unique_across_vehicle_and_built_in_actions() {
    // Bus-only and tram-only keys may share a key (Num * is door 2 and the tram's side).
    let mut seen: Vec<(&str, String)> = Vec::new();
    for (setting, default, _) in binding_settings() {
        if default == "none" {
            continue;
        }
        if let Some((_, other)) = seen
            .iter()
            .find(|(d, o)| *d == default && scope_of(o).overlaps(scope_of(&setting)))
        {
            panic!("{setting} and {other} both default to {default}");
        }
        seen.push((default, setting));
    }
}

#[test]
fn every_action_has_a_row_and_messages_in_both_languages() {
    let (reg, ..) = parts();
    let en = crate::settings_registry::localization("en");
    let de = crate::settings_registry::localization("de");
    for a in action_keys::ACTIONS {
        let d = reg.get(&action_keys::setting_of(a)).expect(a.id);
        assert!(matches!(d.kind, K::Binding));
        for l in [&en, &de] {
            assert_ne!(l.t(d.label_key), d.label_key, "{}", d.label_key);
            assert_ne!(
                l.t(d.description_key),
                d.description_key,
                "{}",
                d.description_key
            );
        }
    }
    for (key, label, desc) in RESET_ROWS {
        assert!(matches!(reg.get(key).expect(key).kind, K::Action));
        for l in [&en, &de] {
            assert_ne!(l.t(label), label);
            assert_ne!(l.t(desc), desc);
        }
    }
}

#[test]
fn a_built_in_action_is_rebound_and_the_conflicts_name_the_holder() {
    let (reg, caps, mut s) = parts();
    let (_, new) = reg
        .set(&mut s, &caps, "controls.ui.help", "ctrl+h")
        .unwrap();
    assert_eq!(new, "Ctrl+H");
    // Another built-in action's key.
    let e = reg
        .set(&mut s, &caps, "controls.ui.theme", "F6")
        .unwrap_err();
    assert!(e.to_string().contains("map.full_map"), "{e}");
    // A vehicle action's key.
    let e = reg
        .set(&mut s, &caps, "controls.ui.theme", "L")
        .unwrap_err();
    assert!(e.to_string().contains("vehicle.lights"), "{e}");
    // The key help left is free for another action now.
    assert!(reg.set(&mut s, &caps, "controls.ui.theme", "F1").is_ok());
    // And a vehicle action may take a function key a built-in action does not hold any more.
    reg.set(&mut s, &caps, "controls.map.full_map", "Ctrl+M")
        .unwrap();
    assert!(reg
        .set(&mut s, &caps, "controls.vehicle.hazard", "F6")
        .is_ok());
    // A key the interface keeps stays refused.
    assert!(reg
        .set(&mut s, &caps, "controls.ui.help", "Escape")
        .is_err());
}

#[test]
fn the_registry_follows_the_settings() {
    let mut app = App::new();
    app.add_plugins(ActionKeysPlugin)
        .insert_resource(SettingsRegistry::with_builtin())
        .insert_resource(GameSettings::default());
    app.update();
    assert_eq!(app.world().resource::<ActionKeys>().label("help"), "F1");
    let reg = app.world().resource::<SettingsRegistry>().clone();
    reg.set(
        &mut app.world_mut().resource_mut::<GameSettings>(),
        &SettingsCapabilities::default(),
        "controls.ui.help",
        "Ctrl+H",
    )
    .unwrap();
    app.update();
    assert_eq!(app.world().resource::<ActionKeys>().label("help"), "Ctrl+H");
}

#[test]
fn reset_puts_one_group_or_everything_back() {
    let (reg, caps, mut s) = parts();
    reg.set(&mut s, &caps, "controls.ui.help", "Ctrl+H")
        .unwrap();
    reg.set(&mut s, &caps, "controls.map.full_map", "Ctrl+M")
        .unwrap();
    reg.set(&mut s, &caps, "controls.vehicle.lights", "Ctrl+G")
        .unwrap();
    let changed = reset_bindings(&reg, &mut s, Some("map"));
    assert_eq!(changed.len(), 1);
    assert_eq!(reg.text(&s, "controls.map.full_map").unwrap(), "F6");
    assert_eq!(
        reg.text(&s, "controls.ui.help").unwrap(),
        "Ctrl+H",
        "other groups stay"
    );
    let changed = reset_bindings(&reg, &mut s, None);
    assert_eq!(changed.len(), 2);
    for (setting, default, _) in binding_settings() {
        assert_eq!(reg.text(&s, &setting).unwrap(), default, "{setting}");
    }
    assert!(
        reset_bindings(&reg, &mut s, None).is_empty(),
        "nothing left to reset"
    );
}

#[test]
fn a_reset_can_swap_keys_that_conflict_in_between() {
    let (reg, caps, mut s) = parts();
    // help takes F6's chord after full_map moved away; the reset must restore both.
    reg.set(&mut s, &caps, "controls.map.full_map", "Ctrl+M")
        .unwrap();
    reg.set(&mut s, &caps, "controls.ui.help", "F6").unwrap();
    reset_bindings(&reg, &mut s, None);
    assert_eq!(reg.text(&s, "controls.ui.help").unwrap(), "F1");
    assert_eq!(reg.text(&s, "controls.map.full_map").unwrap(), "F6");
}
