//! Main menu screen hierarchy, responsive container layout, and branding nodes.

use bevy::prelude::*;
use opts_gui::prelude::*;
use std::path::Path;

use crate::menu::{
    background::spawn_menu_background, entries::spawn_menu_entries, glass::GlassSurface,
    theme_toggle::spawn_theme_toggle,
};

/// Root marker component for all UI entities spawned as part of the main menu.
#[derive(Component)]
pub struct MainMenuRoot;

/// Marker component for the interactive sidebar container.
#[derive(Component)]
pub struct MenuCard;

/// Spawns the complete main menu screen hierarchy featuring a docked left sidebar.
pub fn spawn_main_menu(
    commands: &mut Commands,
    map_dir: Option<&Path>,
    images: &mut ResMut<Assets<Image>>,
    theme: &MaterialTheme,
) {
    // Top-level full-viewport root node
    let root = commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                ..default()
            },
            MainMenuRoot,
        ))
        .id();

    // 1. Map-backed background with contrast scrim (z=0)
    let bg_entity = spawn_menu_background(commands, map_dir, images, theme);
    commands.entity(root).add_child(bg_entity);

    // 2. Floating theme toggle (top-right, z=2)
    let toggle_entity = spawn_theme_toggle(commands, theme);
    commands.entity(root).add_child(toggle_entity);

    // 3. Floating version label (bottom-right, z=2)
    let version_entity = commands
        .spawn((NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                right: Val::Px(24.0),
                ..default()
            },
            z_index: ZIndex::Local(2),
            ..default()
        },))
        .with_children(|parent| {
            parent.spawn((
                TextBundle::from_section(
                    "OPTS v0.1.0-alpha · Clean-Room Architecture",
                    TextStyle {
                        font_size: 12.0,
                        color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::LabelSmall),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));
        })
        .id();
    commands.entity(root).add_child(version_entity);

    // 4. Docked Left Navigation Sidebar (z=2)
    let sidebar_width = Val::Px(280.0);
    let sidebar_entity = commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    width: sidebar_width,
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect {
                        left: Val::Px(20.0),
                        right: Val::Px(20.0),
                        top: Val::Px(24.0),
                        bottom: Val::Px(24.0),
                    },
                    row_gap: Val::Px(14.0),
                    align_items: AlignItems::Stretch,
                    border: UiRect {
                        right: Val::Px(1.0),
                        ..default()
                    },
                    ..default()
                },
                border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                z_index: ZIndex::Local(2),
                ..default()
            },
            MenuCard,
            GlassSurface::new(ColorRole::SurfaceContainerLowest, 0.82),
            BorderRole(ColorRole::OutlineVariant),
            ElevationToken(opts_gui::theme::elevation::ElevationLevel::Level2),
        ))
        .id();

    // Populate sidebar buttons: shutdown, load last game, start new game, settings, credits
    spawn_menu_entries(sidebar_entity, commands);
    commands.entity(root).add_child(sidebar_entity);

    // 5. Right Hero Branding and Overview Panel (z=1)
    let hero_entity = commands
        .spawn((NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(300.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                padding: UiRect::axes(Val::Px(48.0), Val::Px(32.0)),
                row_gap: Val::Px(20.0),
                ..default()
            },
            z_index: ZIndex::Local(1),
            ..default()
        },))
        .with_children(|hero_parent| {
            // Title & Subtitle
            hero_parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|brand| {
                    brand.spawn((
                        TextBundle::from_section(
                            "OPTS",
                            TextStyle {
                                font_size: 64.0,
                                color: theme.scheme.get(ColorRole::Primary),
                                ..default()
                            },
                        ),
                        TypeStyleToken(TypeScale::DisplayLarge),
                        ForegroundRole(ColorRole::Primary),
                    ));

                    brand.spawn((
                        TextBundle::from_section(
                            "Open Public Transport Simulator",
                            TextStyle {
                                font_size: 18.0,
                                color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                                ..default()
                            },
                        ),
                        TypeStyleToken(TypeScale::TitleMedium),
                        ForegroundRole(ColorRole::OnSurfaceVariant),
                    ));
                });

            // Status Card
            hero_parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            max_width: Val::Px(480.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(20.0)),
                            row_gap: Val::Px(10.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                        ..default()
                    },
                    GlassSurface::new(ColorRole::SurfaceContainerLow, 0.65),
                    RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
                    BorderRole(ColorRole::OutlineVariant),
                ))
                .with_children(|card| {
                    card.spawn((
                        TextBundle::from_section(
                            "Clean-Room Simulation Engine",
                            TextStyle {
                                font_size: 15.0,
                                color: theme.scheme.get(ColorRole::OnSurface),
                                ..default()
                            },
                        ),
                        TypeStyleToken(TypeScale::TitleSmall),
                        ForegroundRole(ColorRole::OnSurface),
                    ));

                    let map_text = if let Some(path) = map_dir {
                        format!("Map Preview: {}", path.display())
                    } else {
                        "No map loaded · Select Start New Game to begin.".to_string()
                    };

                    card.spawn((
                        TextBundle::from_section(
                            map_text,
                            TextStyle {
                                font_size: 13.0,
                                color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                                ..default()
                            },
                        ),
                        TypeStyleToken(TypeScale::BodySmall),
                        ForegroundRole(ColorRole::OnSurfaceVariant),
                    ));

                    card.spawn((
                        TextBundle::from_section(
                            "Storage: SQLite (WAL) · UI: Material Design 3 Glassmorphism",
                            TextStyle {
                                font_size: 11.0,
                                color: theme.scheme.get(ColorRole::Outline),
                                ..default()
                            },
                        ),
                        TypeStyleToken(TypeScale::LabelSmall),
                        ForegroundRole(ColorRole::Outline),
                    ));
                });
        })
        .id();

    commands.entity(root).add_child(hero_entity);
}

/// System that despawns all main menu entities when exiting `AppState::MainMenu`.
pub fn cleanup_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuRoot>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
