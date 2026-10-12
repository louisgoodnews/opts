//! The text field on top of Bevy's `EditableText`, without a window. Key handling of the
//! `TextInput` widget itself (logical keys, IME, clipboard shortcuts) belongs to Bevy; these tests
//! queue the same `TextEdit`s it would and check everything the library adds.

use bevy::input_focus::InputFocusPlugin;
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit, TextPlugin};
use opts_gui::components::text_field::{M3TextField, TextFieldBuilder, TextFieldClear};
use opts_gui::interaction::events::TextChanged;
use opts_gui::testing::{act, click, focused};
use opts_gui::{actions::UiAction, OptsGuiPlugin};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::input::InputPlugin,
        bevy::window::WindowPlugin {
            primary_window: None,
            ..default()
        },
        TextPlugin,
        InputFocusPlugin,
        OptsGuiPlugin,
    ));
    app.update();
    app
}

/// Applies an edit to the focused input like `TextInput` does for a key press.
fn edit(app: &mut App, edit: TextEdit) {
    let input = focused(app).expect("an input has focus");
    app.world_mut()
        .get_mut::<EditableText>(input)
        .expect("focused entity is the text input")
        .queue_edit(edit);
    app.update();
    app.update();
}

fn type_str(app: &mut App, s: &str) {
    edit(app, TextEdit::Insert(s.into()));
}

fn spawn_field(app: &mut App, text: &str) -> Entity {
    let e = {
        let mut commands = app.world_mut().commands();
        TextFieldBuilder::outlined("Search")
            .with_placeholder("Search 956 vehicles...")
            .with_text(text)
            .spawn(&mut commands)
    };
    app.update();
    app.update();
    e
}

fn value(app: &App, e: Entity) -> String {
    app.world()
        .get::<M3TextField>(e)
        .map(|f| f.text.clone())
        .unwrap_or_default()
}

#[test]
fn typed_characters_reach_the_value() {
    let mut app = app();
    let field = spawn_field(&mut app, "");
    act(&mut app, UiAction::FocusNext);
    assert!(focused(&mut app).is_some(), "the input takes focus");
    // Characters (any layout, umlauts) arrive as text, never as physical key codes.
    type_str(&mut app, "z");
    type_str(&mut app, "ü");
    assert_eq!(value(&app, field), "zü");
}

#[test]
fn typing_sends_text_changed() {
    let mut app = app();
    let field = spawn_field(&mut app, "");
    act(&mut app, UiAction::FocusNext);
    type_str(&mut app, "ab");
    assert_eq!(value(&app, field), "ab");
    let events = app.world().resource::<Messages<TextChanged>>();
    assert!(events
        .get_cursor()
        .read(events)
        .any(|e| e.entity == field && e.text == "ab"));
}

#[test]
fn programmatic_text_reaches_the_input_and_the_placeholder_follows() {
    let mut app = app();
    let field = spawn_field(&mut app, "");
    app.world_mut()
        .get_mut::<M3TextField>(field)
        .expect("field")
        .text = "M29".into();
    app.update();
    app.update();
    act(&mut app, UiAction::FocusNext);
    type_str(&mut app, "x");
    assert_eq!(value(&app, field), "M29x", "caret was moved to the end");
}

#[test]
fn clear_button_empties_the_field_and_refocuses_the_input() {
    let mut app = app();
    let field = spawn_field(&mut app, "abc");
    let clear = app
        .world_mut()
        .query::<(Entity, &TextFieldClear)>()
        .iter(app.world())
        .find(|(_, c)| c.0 == field)
        .map(|(e, _)| e)
        .expect("clear button");
    click(&mut app, clear);
    app.update();
    assert_eq!(value(&app, field), "");
    assert!(focused(&mut app).is_some());
}

#[test]
fn back_leaves_the_field_without_leaving_the_screen() {
    let mut app = app();
    let field = spawn_field(&mut app, "a");
    act(&mut app, UiAction::FocusNext);
    type_str(&mut app, " ");
    assert_eq!(value(&app, field), "a ");
    act(&mut app, UiAction::Back);
    assert_eq!(focused(&mut app), None, "first Back drops focus");
}
