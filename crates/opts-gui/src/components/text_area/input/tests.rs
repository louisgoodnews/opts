//! Unit tests of `input`.

use super::*;
use bevy::input::InputPlugin;

fn app_with_area() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, InputPlugin))
        .add_message::<TextAreaChanged>()
        .insert_non_send(TextClipboard::in_memory())
        .add_systems(Update, text_area_input_system);
    let e = app
        .world_mut()
        .spawn(TextArea {
            focused: true,
            ..default()
        })
        .id();
    (app, e)
}

fn press(app: &mut App, key_code: KeyCode, logical_key: Key) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
}

#[test]
fn typing_enter_and_backspace_edit_the_buffer() {
    let (mut app, e) = app_with_area();
    press(&mut app, KeyCode::KeyU, Key::Character("ü".into()));
    press(&mut app, KeyCode::Enter, Key::Enter);
    press(&mut app, KeyCode::KeyX, Key::Character("x".into()));
    assert_eq!(
        app.world().get::<TextArea>(e).map(TextArea::value),
        Some("ü\nx")
    );
    press(&mut app, KeyCode::Backspace, Key::Backspace);
    assert_eq!(
        app.world().get::<TextArea>(e).map(TextArea::value),
        Some("ü\n")
    );
    let events = app.world().resource::<Messages<TextAreaChanged>>();
    assert!(!events.is_empty());
}

#[test]
fn unfocused_areas_ignore_keys() {
    let (mut app, e) = app_with_area();
    if let Some(mut a) = app.world_mut().get_mut::<TextArea>(e) {
        a.focused = false;
    }
    press(&mut app, KeyCode::KeyA, Key::Character("a".into()));
    assert_eq!(
        app.world().get::<TextArea>(e).map(TextArea::value),
        Some("")
    );
}

#[test]
fn copy_and_paste_round_trip_through_the_clipboard() {
    let (mut app, e) = app_with_area();
    if let Some(mut a) = app.world_mut().get_mut::<TextArea>(e) {
        a.buffer.set_text("abc");
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    press(&mut app, KeyCode::KeyA, Key::Character("a".into()));
    press(&mut app, KeyCode::KeyX, Key::Character("x".into()));
    assert_eq!(
        app.world().get::<TextArea>(e).map(TextArea::value),
        Some("")
    );
    press(&mut app, KeyCode::KeyV, Key::Character("v".into()));
    assert_eq!(
        app.world().get::<TextArea>(e).map(TextArea::value),
        Some("abc")
    );
}
