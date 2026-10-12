//! Main menu interactive button entries and callback actions for the left navigation sidebar.

use bevy::app::AppExit;
use bevy::prelude::*;
use opts_gui::l10n::keys::menu as keys;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::menu::glass::GlassSurface;

/// Marker component for the shutdown / quit button entry.
#[derive(Component)]
pub struct QuitButton;

/// Spawns the canonical sidebar buttons in the user's requested order:
/// 1. (top left corner) shutdown
/// 2. load last game
/// 3. start new game
/// 4. settings
/// 5. Credits
pub fn spawn_menu_entries(sidebar_entity: Entity, commands: &mut Commands) {
    let mut shutdown_btn = Entity::PLACEHOLDER;
    let mut load_btn = Entity::PLACEHOLDER;
    let mut start_btn = Entity::PLACEHOLDER;
    let mut settings_btn = Entity::PLACEHOLDER;
    let mut credits_btn = Entity::PLACEHOLDER;

    commands.entity(sidebar_entity).with_children(|parent| {
        // 1. (Top Left Corner) Shutdown Button
        shutdown_btn = ButtonBuilder::tonal("Shutdown")
            .with_key(keys::SHUTDOWN)
            .spawn_child(parent);

        // Subtle divider separating top-corner shutdown from game flow
        DividerBuilder::horizontal().spawn_child(parent);

        // 2. Load Last Game Button (Tonal, Disabled if no saved game)
        load_btn = ButtonBuilder::tonal("Load Last Game")
            .with_key(keys::LOAD_LAST_GAME)
            .on_click(|cmd, _entity| {
                tracing::info!("Navigating to Load Game");
                cmd.insert_resource(NextState::Pending(AppState::LoadGame));
            })
            .spawn_child(parent);

        // 3. Start New Game Button (Primary Filled Highlight)
        start_btn = ButtonBuilder::filled("Start New Game")
            .with_key(keys::START_NEW_GAME)
            .on_click(|cmd, _entity| {
                tracing::info!("Navigating to Vehicle Selection (New Game)");
                cmd.insert_resource(NextState::Pending(AppState::VehicleSelection));
            })
            .spawn_child(parent);

        // 4. Settings Button
        settings_btn = ButtonBuilder::outlined("Settings")
            .with_key(keys::SETTINGS)
            .on_click(|cmd, _entity| {
                tracing::info!("Navigating to Settings");
                cmd.insert_resource(NextState::Pending(AppState::Settings));
            })
            .spawn_child(parent);

        // 5. Credits Button
        credits_btn = ButtonBuilder::text("Credits")
            .with_key(keys::CREDITS)
            .on_click(|cmd, _entity| {
                tracing::info!("Navigating to Credits");
                cmd.insert_resource(NextState::Pending(AppState::Credits));
            })
            .spawn_child(parent);
    });

    // Glassmorphic surfaces (the library state layer takes its tint from the glass colour) and
    // stable ids; "Start New Game" is the safest sensible default focus (never Shutdown).
    let glass = [
        (
            shutdown_btn,
            "menu.shutdown",
            ColorRole::ErrorContainer,
            0.45,
        ),
        (
            load_btn,
            "menu.load-last-game",
            ColorRole::SecondaryContainer,
            0.55,
        ),
        (start_btn, "menu.start-new-game", ColorRole::Primary, 0.85),
        (
            settings_btn,
            "menu.settings",
            ColorRole::SurfaceContainerHigh,
            0.60,
        ),
        (
            credits_btn,
            "menu.credits",
            ColorRole::SurfaceContainerHigh,
            0.45,
        ),
    ];
    for (button, id, role, alpha) in glass {
        commands.entity(button).remove::<BackgroundRole>().insert((
            GlassSurface::new(role, alpha),
            StateLayerRole(opts_gui::interaction::layer::on_color(role)),
            UiId::new(id),
        ));
    }
    commands.entity(shutdown_btn).insert(QuitButton);
    // Visible and marked, not a silently greyed button (decision D-UI-09).
    commands
        .entity(load_btn)
        .insert(NotAvailableYet::new(&opts_gui::not_yet::LOAD_LAST_GAME));
    commands.entity(start_btn).insert(DefaultFocus);
}

/// System that exits the application when the shutdown button is clicked.
pub fn quit_button_action_system(
    mut events: MessageReader<ButtonClicked>,
    quit_query: Query<&QuitButton>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for event in events.read() {
        if quit_query.contains(event.0) {
            tracing::info!("Shutting down OPTS via Shutdown button");
            app_exit.write(AppExit::Success);
        }
    }
}
