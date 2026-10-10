//! Systems handling interactions and persistence in the settings screen.

use bevy::prelude::*;
use opts_gui::components::toast::ToastBuilder;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::catalogue_service::{spawn_discovery, CatalogueService};
use crate::settings::GameSettings;
use crate::settings_screen::types::{
    BackToMenuButton, ColorSwatchButton, DarkModeToggleButton, DiscoveryIntervalButton,
    RunDiscoveryNowButton, SettingsScreenRoot, ThemePresetButton,
};

/// System returning to the main menu when the back button is clicked.
pub fn handle_back_to_menu(
    mut events: MessageReader<ButtonClicked>,
    back_query: Query<&BackToMenuButton>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for event in events.read() {
        if back_query.contains(event.0) {
            tracing::info!("Returning to Main Menu from Settings");
            next_state.set(AppState::MainMenu);
        }
    }
}

/// System handling preset theme clicks and saving to SQLite.
pub fn handle_theme_preset_clicks(
    mut events: MessageReader<ButtonClicked>,
    preset_query: Query<&ThemePresetButton>,
    mut theme: ResMut<MaterialTheme>,
    mut settings: ResMut<GameSettings>,
) {
    for event in events.read() {
        if let Ok(preset_btn) = preset_query.get(event.0) {
            tracing::info!(preset = preset_btn.preset_name, "Applied theme preset");
            settings.active_theme = preset_btn.preset_name.to_string();

            let mut scheme = if settings.is_dark_theme {
                ColorScheme::dark()
            } else {
                ColorScheme::light()
            };

            match preset_btn.preset_name {
                "Amber" => {
                    scheme.set(ColorRole::Primary, Color::srgb(0.96, 0.62, 0.04));
                    scheme.set(ColorRole::Secondary, Color::srgb(0.85, 0.45, 0.10));
                }
                "Violet" => {
                    scheme.set(ColorRole::Primary, Color::srgb(0.55, 0.36, 0.96));
                    scheme.set(ColorRole::Secondary, Color::srgb(0.40, 0.20, 0.80));
                }
                "Emerald" => {
                    scheme.set(ColorRole::Primary, Color::srgb(0.06, 0.72, 0.51));
                    scheme.set(ColorRole::Secondary, Color::srgb(0.04, 0.55, 0.38));
                }
                "Teal" => {
                    scheme.set(ColorRole::Primary, Color::srgb(0.08, 0.65, 0.75));
                    scheme.set(ColorRole::Secondary, Color::srgb(0.05, 0.50, 0.60));
                }
                _ => {}
            }

            theme.scheme = scheme;
            if let Err(err) = settings.save() {
                tracing::warn!(err = ?err, "Failed to persist theme preset to SQLite database");
            }
        }
    }
}

/// System handling per-widget color swatch clicks and saving to SQLite.
pub fn handle_color_swatch_clicks(
    mut events: MessageReader<ButtonClicked>,
    swatch_query: Query<&ColorSwatchButton>,
    mut theme: ResMut<MaterialTheme>,
    mut settings: ResMut<GameSettings>,
) {
    for event in events.read() {
        if let Ok(swatch) = swatch_query.get(event.0) {
            tracing::info!(
                widget = swatch.widget_type,
                role = swatch.role_name,
                hex = swatch.hex,
                "Applied per-widget color customization"
            );

            settings.set_widget_color(swatch.widget_type, swatch.role_name, swatch.hex);

            match swatch.role_name {
                "Primary" => theme.scheme.set(ColorRole::Primary, swatch.color),
                "SecondaryContainer" => theme
                    .scheme
                    .set(ColorRole::SecondaryContainer, swatch.color),
                _ => {}
            }

            if let Err(err) = settings.save() {
                tracing::warn!(err = ?err, "Failed to persist widget color to SQLite database");
            }
        }
    }
}

/// System handling Dark / Light theme mode toggle and saving to SQLite.
pub fn handle_dark_mode_toggle_clicks(
    mut events: MessageReader<ButtonClicked>,
    toggle_query: Query<&DarkModeToggleButton>,
    mut theme: ResMut<MaterialTheme>,
    mut settings: ResMut<GameSettings>,
    mut text_query: Query<&mut Text>,
    toggle_btn_query: Query<&Children, With<DarkModeToggleButton>>,
) {
    for event in events.read() {
        if toggle_query.contains(event.0) {
            settings.is_dark_theme = !settings.is_dark_theme;
            tracing::info!(
                dark = settings.is_dark_theme,
                "Toggled theme mode in settings"
            );

            theme.scheme = if settings.is_dark_theme {
                ColorScheme::dark()
            } else {
                ColorScheme::light()
            };

            // Update button label with clean text (no missing glyphs)
            if let Ok(children) = toggle_btn_query.get(event.0) {
                for &child in children {
                    if let Ok(mut text) = text_query.get_mut(child) {
                        text.0 = if settings.is_dark_theme {
                            "Dark Mode (Active)".to_string()
                        } else {
                            "Light Mode (Active)".to_string()
                        };
                    }
                }
            }

            if let Err(err) = settings.save() {
                tracing::warn!(err = ?err, "Failed to persist dark theme toggle to SQLite");
            }
        }
    }
}

/// System handling discovery interval frequency selection and saving to SQLite.
pub fn handle_discovery_interval_clicks(
    mut events: MessageReader<ButtonClicked>,
    interval_query: Query<&DiscoveryIntervalButton>,
    mut card_query: Query<(
        &DiscoveryIntervalButton,
        &mut BorderColor,
        &mut BackgroundColor,
    )>,
    mut settings: ResMut<GameSettings>,
    theme: Res<MaterialTheme>,
) {
    let mut selected = None;
    for event in events.read() {
        if let Ok(btn) = interval_query.get(event.0) {
            selected = Some(btn.interval_days);
        }
    }

    if let Some(days) = selected {
        settings.discovery_interval_days = days;
        tracing::info!(days = days, "Updated vehicle discovery interval");

        for (btn, mut border, mut bg) in &mut card_query {
            let is_active = btn.interval_days == days;
            *border = BorderColor::all(if is_active {
                theme.scheme.get(ColorRole::Primary)
            } else {
                theme.scheme.get(ColorRole::OutlineVariant)
            });
            *bg = BackgroundColor(if is_active {
                theme.scheme.get(ColorRole::PrimaryContainer)
            } else {
                theme.scheme.get(ColorRole::SurfaceContainerHigh)
            });
        }

        if let Err(err) = settings.save() {
            tracing::warn!(err = ?err, "Failed to persist discovery interval to SQLite");
        }
    }
}

/// System handling manual "Run Discovery Now" button click.
pub fn handle_run_discovery_clicks(
    mut commands: Commands,
    mut events: MessageReader<ButtonClicked>,
    trigger_query: Query<&RunDiscoveryNowButton>,
    service: Option<Res<CatalogueService>>,
) {
    for event in events.read() {
        if trigger_query.contains(event.0) {
            tracing::info!("Starting immediate catalogue discovery from Settings menu");
            if let Some(service) = &service {
                spawn_discovery(service);
                ToastBuilder::new("OMSI catalogue discovery started in the background.")
                    .with_duration(3.0)
                    .spawn(&mut commands);
            } else {
                ToastBuilder::new("No OMSI installation found; cannot run discovery.")
                    .with_duration(4.0)
                    .spawn(&mut commands);
            }
        }
    }
}

/// System that despawns all settings screen entities on exit.
pub fn cleanup_settings_screen(
    mut commands: Commands,
    query: Query<Entity, With<SettingsScreenRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

/// Escape key listener returning to the Main Menu.
pub fn escape_to_menu_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::MainMenu);
    }
}
