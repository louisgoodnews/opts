//! Mini-map tests: the keys change the settings.

use super::*;

#[test]
fn the_keys_change_the_settings() {
    let mut app = map_app();
    let press = |app: &mut App, keys: &[KeyCode]| {
        use bevy::input::keyboard::{Key, KeyboardInput};
        use bevy::input::ButtonState;
        for state in [ButtonState::Pressed, ButtonState::Released] {
            for k in keys {
                app.world_mut().write_message(KeyboardInput {
                    key_code: *k,
                    logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                    state,
                    text: None,
                    repeat: false,
                    window: Entity::PLACEHOLDER,
                });
            }
            for _ in 0..2 {
                app.update();
            }
        }
    };
    let config = |app: &App| MiniMapConfig::read(app.world().resource::<GameSettings>());
    assert_eq!(config(&app).zoom_m, 200.0);
    press(&mut app, &[KeyCode::Equal]);
    assert_eq!(config(&app).zoom_m, 100.0, "+ zooms in");
    press(&mut app, &[KeyCode::Equal]);
    assert_eq!(config(&app).zoom_m, 100.0, "stops at 100 m");
    press(&mut app, &[KeyCode::Minus]);
    press(&mut app, &[KeyCode::Minus]);
    assert_eq!(config(&app).zoom_m, 500.0, "- zooms out");
    press(&mut app, &[KeyCode::ShiftLeft, KeyCode::F7]);
    assert_eq!(config(&app).orientation, Orientation::North);
    press(&mut app, &[KeyCode::F7]);
    assert!(!config(&app).enabled, "F7 hides it");
    press(&mut app, &[KeyCode::F7]);
    assert!(config(&app).enabled, "and shows it again");
    assert_eq!(
        config(&app).zoom_mode,
        minimap::ZoomMode::Speed,
        "speed zoom is the default"
    );
    press(&mut app, &[KeyCode::ControlLeft, KeyCode::F7]);
    assert_eq!(
        config(&app).zoom_mode,
        minimap::ZoomMode::Fixed,
        "Ctrl+F7 fixes the zoom"
    );
    assert!(config(&app).enabled, "and does not hide the map");
    press(&mut app, &[KeyCode::ControlLeft, KeyCode::F7]);
    assert_eq!(config(&app).zoom_mode, minimap::ZoomMode::Speed, "and back");
}
