//! Integration tests for the shell rail: generation from the registry, content hosting and rail jumps.
use super::*;

#[test]
fn the_rail_is_generated_from_the_registry_in_order() {
    let mut app = shell_app();
    let mut items: Vec<(&str, bool)> = app
        .world_mut()
        .query::<(&RailItem, Has<NotAvailableYet>)>()
        .iter(app.world())
        .map(|(i, n)| (i.id, n))
        .collect();
    items.sort_by_key(|(id, _)| {
        ["home", "drive", "library", "workshop", "settings", "help"]
            .iter()
            .position(|x| x == id)
    });
    assert_eq!(
        items,
        [
            ("home", false),
            ("drive", false),
            ("library", true),
            ("workshop", true),
            ("settings", false),
            ("help", false),
        ]
    );
}

#[test]
fn screens_are_hosted_in_the_content_area_of_the_shell() {
    let mut app = shell_app();
    let content = app.world().resource::<ShellContent>().0;
    assert!(content.is_some());
    assert_eq!(
        app.world_mut()
            .query::<&ShellRoot>()
            .iter(app.world())
            .count(),
        1
    );
    let home = app
        .world_mut()
        .query_filtered::<Entity, With<opts_game::home::HomeRoot>>()
        .single(app.world())
        .expect("one home page");
    assert_eq!(
        app.world().get::<ChildOf>(home).map(ChildOf::parent),
        content
    );
}

#[test]
fn a_rail_jump_changes_the_screen_and_clears_the_stack() {
    let mut app = shell_app();
    go(&mut app, Navigate::push("drive"));
    assert_eq!(state(&app), AppState::VehicleSelection);
    go(&mut app, Navigate::to("help"));
    assert_eq!(state(&app), AppState::Help);
    back(&mut app);
    assert_eq!(
        state(&app),
        AppState::MainMenu,
        "stack was cleared: Back goes Home"
    );
}
