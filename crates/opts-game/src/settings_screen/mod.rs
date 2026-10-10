//! Interactive Settings menu screen utilizing the Material Design 3 Notebook widget.
//!
//! # Architecture & Features
//! - Uses [`opts_gui::components::notebook::NotebookBuilder`] to organize settings into tabs.
//! - **UI Tab:** Per-widget color role customizers, theme preset selections, and dark mode toggle.
//! - **General Tab:** Displays SQLite storage connection, discovery frequency settings, and manual discovery trigger.
//! - **Display & Audio Tab:** Resolution, window mode, and volume presets.
//! - Persists all active selections to SQLite `settings.db` while leaving `settings.toml` as an immutable reference.

pub mod display_tab;
pub mod general_tab;
pub mod systems;
pub mod types;
pub mod ui_tab;

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::menu::glass::GlassSurface;
use crate::settings::GameSettings;

pub use display_tab::spawn_display_tab;
pub use general_tab::spawn_general_tab;
pub use systems::{
    cleanup_settings_screen, escape_to_menu_system, handle_back_to_menu,
    handle_color_swatch_clicks, handle_dark_mode_toggle_clicks, handle_discovery_interval_clicks,
    handle_run_discovery_clicks, handle_theme_preset_clicks,
};
pub use types::{
    BackToMenuButton, ColorSwatchButton, DarkModeToggleButton, DiscoveryIntervalButton,
    RunDiscoveryNowButton, SettingsScreenRoot, ThemePresetButton,
};
pub use ui_tab::spawn_ui_tab;

/// Plugin managing the interactive Settings screen.
#[derive(Default)]
pub struct SettingsScreenPlugin;

impl Plugin for SettingsScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Settings), spawn_settings_screen)
            .add_systems(OnExit(AppState::Settings), cleanup_settings_screen)
            .add_systems(
                Update,
                (
                    handle_back_to_menu,
                    handle_theme_preset_clicks,
                    handle_color_swatch_clicks,
                    handle_dark_mode_toggle_clicks,
                    handle_discovery_interval_clicks,
                    handle_run_discovery_clicks,
                    escape_to_menu_system,
                )
                    .run_if(in_state(AppState::Settings))
                    .run_if(opts_bevy::reporting::not_modal_active),
            );
    }
}

/// Spawns the complete Settings screen with the tabbed Notebook widget.
pub fn spawn_settings_screen(
    mut commands: Commands,
    theme: Res<MaterialTheme>,
    settings: Res<GameSettings>,
) {
    // 1. Fullscreen Root Container
    let root = commands
        .spawn((
            (
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(24.0)),
                    row_gap: Val::Px(16.0),
                    ..default()
                },
                BackgroundColor(theme.scheme.get(ColorRole::Surface)),
            ),
            SettingsScreenRoot,
        ))
        .id();

    // 2. Header Bar with Title and Back Button
    commands.entity(root).with_children(|header_parent| {
        header_parent
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Px(56.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect {
                        bottom: Val::Px(1.0),
                        ..default()
                    },
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                    ..default()
                },
                BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            ))
            .with_children(|bar| {
                // Title
                bar.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                })
                .with_children(|title_box| {
                    title_box.spawn((
                        (
                            Text::new("Application Settings"),
                            TextFont {
                                font_size: FontSize::Px(26.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::Primary)),
                        ),
                        TypeStyleToken(TypeScale::HeadlineSmall),
                        ForegroundRole(ColorRole::Primary),
                    ));

                    title_box.spawn((
                        (
                            Text::new("Preferences, UI Colorization & Theme Configuration"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                        ),
                        TypeStyleToken(TypeScale::LabelMedium),
                        ForegroundRole(ColorRole::OnSurfaceVariant),
                    ));
                });

                // Back Button
                bar.spawn((
                    (
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(18.0), Val::Px(10.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                        BackgroundColor(theme.scheme.get(ColorRole::SecondaryContainer)),
                    ),
                    BackToMenuButton,
                    GlassSurface::new(ColorRole::SecondaryContainer, 0.70),
                    RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
                    Focusable { tab_index: 0 },
                ))
                .with_children(|btn| {
                    btn.spawn((
                        (
                            Text::new("< Back to Main Menu"),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::OnSecondaryContainer)),
                        ),
                        TypeStyleToken(TypeScale::LabelLarge),
                        ForegroundRole(ColorRole::OnSecondaryContainer),
                    ));
                });
            });
    });

    // 3. Notebook Widget with 3 Categories
    let (notebook_root, panels) = NotebookBuilder::new()
        .with_tabs(vec![
            "UI & Themes".into(),
            "General & Storage".into(),
            "Display & Sound".into(),
        ])
        .with_active(0)
        .spawn_with_panels(&mut commands);

    commands.entity(notebook_root).insert(Node {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        width: Val::Percent(100.0),
        height: Val::Auto,
        flex_grow: 1.0,
        flex_shrink: 1.0,
        min_height: Val::Px(0.0),
        overflow: Overflow::clip(),
        ..default()
    });

    commands.entity(root).add_child(notebook_root);

    // Tab 0: UI & Themes Panel
    if let Some(&ui_panel) = panels.first() {
        commands.entity(ui_panel).with_children(|tab| {
            spawn_ui_tab(tab, &theme, &settings);
        });
    }

    // Tab 1: General & Storage Panel
    if let Some(&gen_panel) = panels.get(1) {
        commands.entity(gen_panel).with_children(|tab| {
            spawn_general_tab(tab, &theme, &settings);
        });
    }

    // Tab 2: Display & Sound Panel
    if let Some(&disp_panel) = panels.get(2) {
        commands.entity(disp_panel).with_children(|tab| {
            spawn_display_tab(tab, &theme);
        });
    }
}
