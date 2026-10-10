//! Main menu interactive button entries and callback actions for the left navigation sidebar.

use bevy::app::AppExit;
use bevy::prelude::*;
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
        shutdown_btn = ButtonBuilder::tonal("Shutdown").spawn_child(parent);

        // Subtle divider separating top-corner shutdown from game flow
        DividerBuilder::horizontal().spawn_child(parent);

        // 2. Load Last Game Button (Tonal, Disabled if no saved game)
        load_btn = ButtonBuilder::tonal("Load Last Game")
            .with_disabled(true)
            .on_click(|cmd, _entity| {
                tracing::info!("Navigating to Load Game");
                cmd.insert_resource(NextState::Pending(AppState::LoadGame));
            })
            .spawn_child(parent);

        // 3. Start New Game Button (Primary Filled Highlight)
        start_btn = ButtonBuilder::filled("Start New Game")
            .on_click(|cmd, _entity| {
                tracing::info!("Navigating to Vehicle Selection (New Game)");
                cmd.insert_resource(NextState::Pending(AppState::VehicleSelection));
            })
            .spawn_child(parent);

        // 4. Settings Button
        settings_btn = ButtonBuilder::outlined("Settings")
            .on_click(|cmd, _entity| {
                tracing::info!("Navigating to Settings");
                cmd.insert_resource(NextState::Pending(AppState::Settings));
            })
            .spawn_child(parent);

        // 5. Credits Button
        credits_btn = ButtonBuilder::text("Credits")
            .on_click(|cmd, _entity| {
                tracing::info!("Navigating to Credits");
                cmd.insert_resource(NextState::Pending(AppState::Credits));
            })
            .spawn_child(parent);
    });

    // Apply glassmorphic surface overrides and sequential tab indices
    commands
        .entity(shutdown_btn)
        .remove::<BackgroundRole>()
        .insert((
            QuitButton,
            GlassSurface::new(ColorRole::ErrorContainer, 0.45),
            Focusable { tab_index: 0 },
        ));

    commands
        .entity(load_btn)
        .remove::<BackgroundRole>()
        .insert((
            GlassSurface::new(ColorRole::SecondaryContainer, 0.55),
            Focusable { tab_index: 1 },
        ));

    commands
        .entity(start_btn)
        .remove::<BackgroundRole>()
        .insert((
            GlassSurface::new(ColorRole::Primary, 0.85),
            Focusable { tab_index: 2 },
        ));

    commands
        .entity(settings_btn)
        .remove::<BackgroundRole>()
        .insert((
            GlassSurface::new(ColorRole::SurfaceContainerHigh, 0.60),
            Focusable { tab_index: 3 },
        ));

    commands
        .entity(credits_btn)
        .remove::<BackgroundRole>()
        .insert((
            GlassSurface::new(ColorRole::SurfaceContainerHigh, 0.45),
            Focusable { tab_index: 4 },
        ));
}

/// System that exits the application when the shutdown button is clicked.
pub fn quit_button_action_system(
    mut events: EventReader<ButtonClicked>,
    quit_query: Query<&QuitButton>,
    mut app_exit: EventWriter<AppExit>,
) {
    for event in events.read() {
        if quit_query.contains(event.0) {
            tracing::info!("Shutting down OPTS via Shutdown button");
            app_exit.send(AppExit::Success);
        }
    }
}
