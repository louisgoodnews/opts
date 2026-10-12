//! The application shell: rail from the registry, availability, navigation, back stack, Esc.

use bevy::prelude::*;
use opts_game::shell::frame::{RailItem, ShellRoot};
use opts_game::shell::{
    Navigate, NavigateBack, NavigationRefused, PendingDeepLink, ShellContent, ShellPlugin,
};
use opts_game::{app_state::AppState, help::HelpPlugin, home::HomePlugin, settings::GameSettings};
use opts_gui::not_yet::NotAvailableYet;
use opts_gui::prelude::*;

fn shell_app() -> App {
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
        .add_plugins(ShellPlugin)
        .add_plugins(HomePlugin)
        .add_plugins(HelpPlugin);
    app.update(); // start-up: the shell frame exists before any screen is built
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    for _ in 0..4 {
        app.update();
    }
    app
}

fn state(app: &App) -> AppState {
    *app.world().resource::<State<AppState>>().get()
}

fn go(app: &mut App, message: Navigate) {
    app.world_mut().write_message(message);
    for _ in 0..3 {
        app.update();
    }
}

fn back(app: &mut App) {
    app.world_mut().write_message(UiAction::Back);
    for _ in 0..3 {
        app.update();
    }
}

fn refusals(app: &mut App) -> Vec<NavigationRefused> {
    let messages = app.world().resource::<Messages<NavigationRefused>>();
    messages.get_cursor().read(messages).cloned().collect()
}

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
