//! Key rebinding: a pressed key is stored, a key another action has is refused, and Esc cancels.

use super::*;

/// Controls: *Change key* waits for the next key press and stores it; a key that another action
/// has is refused with its reason on the page, and Esc cancels.
#[test]
fn a_vehicle_key_is_rebound_by_pressing_it() {
    let (mut app, _, db) = page_app("rebind-keys", "controls");
    let change = "settings/controls/controls.vehicle.lights/change";
    let press = |app: &mut App, key: KeyCode| {
        use bevy::input::keyboard::{Key, KeyboardInput};
        for state in [
            bevy::input::ButtonState::Pressed,
            bevy::input::ButtonState::Released,
        ] {
            app.world_mut().write_message(KeyboardInput {
                key_code: key,
                logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                state,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
            app.update();
        }
    };
    let b = by_id(&mut app, change).expect("change button");
    click(&mut app, b);
    app.update();
    assert_eq!(
        app.world().resource::<SettingsPageState>().rebinding,
        Some("controls.vehicle.lights")
    );
    press(&mut app, KeyCode::KeyJ);
    app.update();
    assert_eq!(text(&app, "controls.vehicle.lights"), "J");
    assert_eq!(stored(&db, "controls.vehicle.lights").as_deref(), Some("J"));
    assert!(app
        .world()
        .resource::<SettingsPageState>()
        .rebinding
        .is_none());
    // The engine start key is taken: refused, the old key stays, the reason is on the page.
    let b = by_id(&mut app, change).expect("change button");
    click(&mut app, b);
    app.update();
    press(&mut app, KeyCode::KeyM);
    app.update();
    assert_eq!(text(&app, "controls.vehicle.lights"), "J");
    let notice = app
        .world()
        .resource::<SettingsPageState>()
        .notice
        .clone()
        .expect("a reason");
    assert!(notice.contains("engine_start"), "{notice}");
    // Esc cancels without a change.
    let b = by_id(&mut app, change).expect("change button");
    click(&mut app, b);
    app.update();
    press(&mut app, KeyCode::Escape);
    app.update();
    assert_eq!(text(&app, "controls.vehicle.lights"), "J");
    assert!(app.world().resource::<SettingsPageState>().notice.is_none());
}
