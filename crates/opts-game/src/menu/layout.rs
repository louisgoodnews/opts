//! Main menu screen hierarchy, responsive container layout, and branding nodes.

use bevy::prelude::*;
use opts_gui::l10n::{keys::menu as keys, L10nArgs, L10nKey};
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
            (Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            }),
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
        .spawn(((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                right: Val::Px(24.0),
                ..default()
            },
            ZIndex(2),
        ),))
        .with_children(|parent| {
            parent.spawn((
                (
                    Text::new(keys::FOOTER),
                    L10nKey(keys::FOOTER),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                ),
                TypeStyleToken(TypeScale::Label),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));
        })
        .id();
    commands.entity(root).add_child(version_entity);

    // 4. Docked Left Navigation Sidebar (z=2)
    let sidebar_width = Val::Px(280.0);
    let sidebar_entity = commands
        .spawn((
            (
                Node {
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
                BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                ZIndex(2),
            ),
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
        .spawn(((
            Node {
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
            ZIndex(1),
        ),))
        .with_children(|hero_parent| {
            // Title & Subtitle
            hero_parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|brand| {
                    brand.spawn((
                        (
                            Text::new(keys::TITLE),
                            L10nKey(keys::TITLE),
                            TextFont {
                                font_size: FontSize::Px(64.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::Primary)),
                        ),
                        TypeStyleToken(TypeScale::Display),
                        ForegroundRole(ColorRole::Primary),
                    ));

                    brand.spawn((
                        (
                            Text::new(keys::SUBTITLE),
                            L10nKey(keys::SUBTITLE),
                            TextFont {
                                font_size: FontSize::Px(18.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                        ),
                        TypeStyleToken(TypeScale::TitleMedium),
                        ForegroundRole(ColorRole::OnSurfaceVariant),
                    ));
                });

            // Status Card
            hero_parent
                .spawn((
                    (
                        Node {
                            max_width: Val::Px(480.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(20.0)),
                            row_gap: Val::Px(10.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                    ),
                    GlassSurface::new(ColorRole::SurfaceContainerLow, 0.65),
                    RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
                    BorderRole(ColorRole::OutlineVariant),
                ))
                .with_children(|card| {
                    card.spawn((
                        (
                            Text::new(keys::CARD_TITLE),
                            L10nKey(keys::CARD_TITLE),
                            TextFont {
                                font_size: FontSize::Px(15.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::OnSurface)),
                        ),
                        TypeStyleToken(TypeScale::BodyMedium),
                        ForegroundRole(ColorRole::OnSurface),
                    ));

                    let (map_key, map_args) = match map_dir {
                        Some(path) => (
                            keys::CARD_MAP_PREVIEW,
                            L10nArgs::one("path", path.display().to_string()),
                        ),
                        None => (keys::CARD_NO_MAP, L10nArgs::default()),
                    };

                    card.spawn((
                        map_args,
                        (
                            Text::new(map_key),
                            L10nKey(map_key),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                        ),
                        TypeStyleToken(TypeScale::Label),
                        ForegroundRole(ColorRole::OnSurfaceVariant),
                    ));

                    card.spawn((
                        (
                            Text::new(keys::CARD_STORAGE),
                            L10nKey(keys::CARD_STORAGE),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::Outline)),
                        ),
                        TypeStyleToken(TypeScale::Label),
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
        commands.entity(entity).despawn();
    }
}
