//! Integration tests for shell navigation: back stack, refused destinations, deep links and dialogs.
use super::*;

#[test]
fn back_returns_along_the_stack_and_stops_at_home() {
    let mut app = shell_app();
    go(&mut app, Navigate::to("settings"));
    go(&mut app, Navigate::push("help"));
    assert_eq!(state(&app), AppState::Help);
    back(&mut app);
    assert_eq!(state(&app), AppState::Settings);
    back(&mut app);
    assert_eq!(state(&app), AppState::MainMenu);
    back(&mut app);
    assert_eq!(state(&app), AppState::MainMenu, "Esc on Home does nothing");
}

#[test]
fn unavailable_and_unknown_destinations_are_refused_with_the_reason() {
    let mut app = shell_app();
    let mut refused = Vec::new();
    for target in ["library", "nowhere"] {
        app.world_mut().write_message(Navigate::to(target));
        app.update();
        app.update();
        for r in refusals(&mut app) {
            if !refused.contains(&r) {
                refused.push(r);
            }
        }
    }
    assert_eq!(state(&app), AppState::MainMenu);
    assert_eq!(refused.len(), 2);
    assert!(matches!(
        refused[0].availability,
        Some(opts_gui::not_yet::Availability::NotYet(p)) if p.id == "nav.library"
    ));
    assert_eq!(refused[1].availability, None);
}

#[test]
fn deep_links_reach_the_page() {
    let mut app = shell_app();
    go(&mut app, Navigate::push("screen:settings/graphics"));
    assert_eq!(state(&app), AppState::Settings);
    assert_eq!(
        app.world().resource::<PendingDeepLink>().0.as_deref(),
        Some("graphics")
    );
}

#[test]
fn back_does_not_navigate_while_a_dialog_is_open() {
    let mut app = shell_app();
    go(&mut app, Navigate::to("settings"));
    let (_dialog, _) = {
        let mut commands = app.world_mut().commands();
        ModalBuilder::new().spawn(&mut commands)
    };
    app.update();
    back(&mut app);
    assert_eq!(state(&app), AppState::Settings);
}

#[test]
fn the_back_message_is_idempotent_at_home() {
    let mut app = shell_app();
    app.world_mut().write_message(NavigateBack);
    app.update();
    assert_eq!(state(&app), AppState::MainMenu);
}
