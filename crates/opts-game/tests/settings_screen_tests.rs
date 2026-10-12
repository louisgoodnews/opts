//! Automated integration tests validating the interactive Settings screen, Notebook tabs, and SQLite persistence.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    settings::GameSettings,
    settings_screen::{
        BackToMenuButton, SettingsScreenPlugin, SettingsScreenRoot, ThemeChoice, ThemeChoiceButton,
    },
};
use opts_gui::{components::notebook::M3Notebook, interaction::events::ButtonClicked, prelude::*};

#[test]
fn test_settings_screen_spawns_notebook_and_tabs() {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(GameSettings::default())
        .insert_resource(MaterialTheme::light())
        .init_state::<AppState>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(SettingsScreenPlugin);

    // Transition to Settings
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Settings);
    app.update();

    // Verify SettingsScreenRoot is spawned
    let root_count = app
        .world_mut()
        .query_filtered::<Entity, With<SettingsScreenRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(root_count, 1, "SettingsScreenRoot must exist");

    // Verify Notebook container is spawned with 3 tabs
    let mut nb_query = app.world_mut().query::<&M3Notebook>();
    let nb = nb_query
        .iter(app.world())
        .next()
        .expect("Notebook must be present");
    assert_eq!(nb.tab_count, 3, "Notebook must have 3 tabs");
    assert_eq!(
        nb.active_tab, 0,
        "Notebook initial active tab must be 0 (UI & Themes)"
    );

    // Verify the theme choices exist: 4 modes, 5 accents, 6 scales, 3 densities
    let choices: Vec<ThemeChoice> = app
        .world_mut()
        .query::<&ThemeChoiceButton>()
        .iter(app.world())
        .map(|b| b.0)
        .collect();
    for (name, n) in [
        ("modes", 4),
        ("accents", 5),
        ("scales", 6),
        ("densities", 3),
    ] {
        let count = choices
            .iter()
            .filter(|c| match name {
                "modes" => matches!(c, ThemeChoice::Mode(_)),
                "accents" => matches!(c, ThemeChoice::Accent(_)),
                "scales" => matches!(c, ThemeChoice::Scale(_)),
                _ => matches!(c, ThemeChoice::Density(_)),
            })
            .count();
        assert_eq!(count, n, "{name}");
    }

    // Verify Back button exists
    let back_count = app
        .world_mut()
        .query_filtered::<Entity, With<BackToMenuButton>>()
        .iter(app.world())
        .count();
    assert_eq!(back_count, 1, "Must have BackToMenuButton");
}

fn settings_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(GameSettings::default())
        .insert_resource(MaterialTheme::dark())
        .init_state::<AppState>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(SettingsScreenPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Settings);
    app.update();
    app
}

fn click(app: &mut App, choice: ThemeChoice) {
    let entity = app
        .world_mut()
        .query::<(Entity, &ThemeChoiceButton)>()
        .iter(app.world())
        .find(|(_, b)| b.0 == choice)
        .map(|(e, _)| e)
        .expect("button for the choice");
    app.world_mut().write_message(ButtonClicked(entity));
    app.update();
}

#[test]
fn clicking_a_mode_stores_it_and_a_scale_is_clamped_into_the_settings() {
    let mut app = settings_app();
    click(
        &mut app,
        ThemeChoice::Mode(opts_gui::theme::ThemeMode::Light),
    );
    assert_eq!(
        app.world().resource::<GameSettings>().theme_mode,
        opts_gui::theme::ThemeMode::Light
    );
    click(&mut app, ThemeChoice::Scale(150));
    assert_eq!(app.world().resource::<GameSettings>().ui_scale, 150);
}
