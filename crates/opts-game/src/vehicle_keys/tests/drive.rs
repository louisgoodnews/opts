//! Settings drive the key bindings of the running app.

use super::*;
use crate::settings::GameSettings;
use crate::settings_registry::{SettingsCapabilities, SettingsRegistry};
use opts_bevy::ScriptKeyBinding;

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
