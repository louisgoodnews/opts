//! Integration tests of the Settings screen (`AppState::Settings`): the page appears, the
//! categories and the back button exist, and a click on a control changes the typed settings.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    settings::GameSettings,
    settings_screen::{BackToMenuButton, SettingsScreenPlugin, SettingsScreenRoot},
};
use opts_gui::{interaction::events::ButtonClicked, prelude::*};

fn settings_app(name: &str) -> App {
    let dir = std::env::temp_dir().join(format!("opts-settings-it-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    let mut settings = GameSettings::default();
    settings.db_path_override = Some(dir.join("settings.db"));
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(settings)
        .insert_resource(MaterialTheme::dark())
        .init_state::<AppState>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(SettingsScreenPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Settings);
    for _ in 0..4 {
        app.update();
    }
    app
}

fn by_id(app: &mut App, id: &str) -> Option<Entity> {
    app.world_mut()
        .query::<(Entity, &UiId)>()
        .iter(app.world())
        .find(|(_, u)| u.0 == id)
        .map(|(e, _)| e)
}

fn click(app: &mut App, id: &str) {
    let e = by_id(app, id).unwrap_or_else(|| panic!("no element {id}"));
    app.world_mut().write_message(ButtonClicked(e));
    for _ in 0..3 {
        app.update();
    }
}

#[test]
fn the_settings_screen_has_the_page_the_categories_and_a_back_button() {
    let mut app = settings_app("spawn");
    let roots = app
        .world_mut()
        .query_filtered::<Entity, With<SettingsScreenRoot>>()
        .iter(app.world())
        .count();
    assert_eq!(roots, 1);
    for cat in [
        "graphics",
        "audio",
        "controls",
        "interface",
        "gameplay",
        "content",
        "advanced",
    ] {
        assert!(
            by_id(&mut app, &format!("settings/category/{cat}")).is_some(),
            "{cat}"
        );
    }
    let back = app
        .world_mut()
        .query_filtered::<Entity, With<BackToMenuButton>>()
        .iter(app.world())
        .count();
    assert_eq!(back, 1);
}

#[test]
fn a_click_on_the_interface_page_changes_the_typed_settings() {
    let mut app = settings_app("click");
    click(&mut app, "settings/category/interface");
    click(&mut app, "settings/interface/theme_mode/option-light");
    assert_eq!(
        app.world().resource::<GameSettings>().theme_mode,
        opts_gui::theme::ThemeMode::Light
    );
    click(&mut app, "settings/interface/ui_scale/up");
    assert_eq!(app.world().resource::<GameSettings>().ui_scale, 125);
    click(&mut app, "settings/interface/theme_mode/reset");
    assert_eq!(
        app.world().resource::<GameSettings>().theme_mode,
        opts_gui::theme::ThemeMode::Dark,
        "Reset returns to the dark default"
    );
}
