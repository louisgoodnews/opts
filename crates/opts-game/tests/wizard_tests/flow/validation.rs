//! Integration tests for wizard validation: disabled Next with a reason, and an empty catalogue filling in.
use super::*;

#[test]
fn an_invalid_choice_disables_next_with_a_reason() {
    let mut app = wizard("invalid");
    app.world_mut().resource_mut::<NewGameSetup>().selected_map = "Nowhere".into();
    for _ in 0..4 {
        app.update();
    }
    let next = find_entity(app.world_mut(), "wizard.footer.next").expect("next");
    let w = app.world();
    assert!(w
        .get::<opts_gui::interaction::state::Disabled>(next)
        .is_some());
    let reason = w
        .get::<opts_gui::interaction::state::DisabledReason>(next)
        .expect("a disabled control says why");
    assert_eq!(reason.0, "Choose a map first");
    press(&mut app, "wizard.footer.next");
    assert_eq!(
        step(&app),
        PlannerStep::Where,
        "a disabled Next does nothing"
    );
    cleanup("invalid");
}

/// P-1014: a wizard opened while the first scan still runs (empty lists) must show the lists once
/// the catalogue arrives, and leaving it afterwards must not touch despawned entities.
#[test]
fn a_wizard_opened_on_an_empty_catalogue_fills_when_the_lists_arrive() {
    let tag = "late";
    let _ = std::fs::remove_dir_all(history_path(tag).parent().expect("dir"));
    let fixture = opts_game::catalogue_service::testing::fixture_lists();
    let mut app = layout_app();
    let frame = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(1280.0),
            height: Val::Px(720.0),
            ..default()
        })
        .id();
    app.insert_resource(opts_game::shell::ShellContent(Some(frame)));
    app.add_plugins(bevy::state::app::StatesPlugin)
        .insert_resource(GameSettings::default())
        .init_state::<AppState>()
        .insert_resource(opts_game::new_game_screen::GameCatalogue::default())
        .insert_resource(PlannerHistory::at(Some(history_path(tag))))
        .insert_resource(ConfigLibrary::at(None))
        .add_plugins(NewGameMenuPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    for _ in 0..8 {
        app.update();
    }
    let before = texts(&mut app);
    assert!(
        !before.iter().any(|x| x.starts_with("Search 42 maps")),
        "empty catalogue: {before:?}"
    );
    *app.world_mut()
        .resource_mut::<opts_game::new_game_screen::types::GameCatalogue>() = fixture;
    for _ in 0..8 {
        app.update();
    }
    let after = texts(&mut app);
    assert!(
        after.iter().any(|x| x.starts_with("Search 42 maps")),
        "the list was built again: {after:?}"
    );
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    for _ in 0..8 {
        app.update();
    }
    cleanup(tag);
}
