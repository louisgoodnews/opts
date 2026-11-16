//! Integration tests for shell layout: rail icons and labels per layout class, and Escape in settings.
use super::*;

#[test]
fn the_rail_shows_icons_only_in_the_compact_layout_class_and_labels_otherwise() {
    use opts_game::shell::frame::{RailWideOnly, ShellRail, RAIL_COMPACT_PX, RAIL_WIDE_PX};
    use opts_gui::theme::LayoutClass;
    let mut app = shell_app();
    let width = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&Node, With<ShellRail>>()
            .single(app.world())
            .expect("one rail")
            .width
    };
    let labels_shown = |app: &mut App| {
        app.world_mut()
            .query_filtered::<&Node, With<RailWideOnly>>()
            .iter(app.world())
            .any(|n| n.display != Display::None)
    };
    *app.world_mut().resource_mut::<LayoutClass>() = LayoutClass::Medium;
    app.update();
    assert_eq!(width(&mut app), Val::Px(RAIL_WIDE_PX));
    assert!(labels_shown(&mut app));
    *app.world_mut().resource_mut::<LayoutClass>() = LayoutClass::Compact;
    app.update();
    assert_eq!(width(&mut app), Val::Px(RAIL_COMPACT_PX));
    assert!(!labels_shown(&mut app), "icons only");
    // every entry keeps a tooltip and a stable id, so it can still be named and operated
    let ids = opts_gui::ui_tree::snapshot(app.world_mut())
        .flatten()
        .iter()
        .filter(|n| n.id.starts_with("shell.nav."))
        .count();
    assert_eq!(ids, 6);
}

#[test]
fn the_escape_key_in_settings_goes_one_step_back_through_the_shell_only() {
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::input::ButtonState;
    let mut app = shell_app();
    go(&mut app, Navigate::to("settings"));
    go(&mut app, Navigate::push("help"));
    go(&mut app, Navigate::push("settings"));
    assert_eq!(state(&app), AppState::Settings);
    let press = |app: &mut App, st: ButtonState| {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
            state: st,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        for _ in 0..3 {
            app.update();
        }
    };
    press(&mut app, ButtonState::Pressed);
    press(&mut app, ButtonState::Released);
    assert_eq!(
        state(&app),
        AppState::Help,
        "one press, one level: the screen has no Esc handler of its own"
    );
}
