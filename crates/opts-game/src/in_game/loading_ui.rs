//! Material Design 3 loading screen UI hierarchy and progress indicators.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::in_game::types::{
    CancelLoadingBtn, InGameEntity, LoadingScreenRoot, ProgressBarFill, ProgressPercentText,
    StageIndicatorText, StartDrivingBtn, StatusTickerText,
};
use crate::menu::glass::GlassSurface;
use crate::new_game_screen::types::NewGameSetup;

/// Spawns the fullscreen Material Design 3 loading overlay.
pub fn spawn_loading_screen_ui(
    commands: &mut Commands,
    theme: &MaterialTheme,
    setup: &NewGameSetup,
) {
    let root = commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(Val::Px(32.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.06, 0.08, 0.11, 0.98)),
                z_index: ZIndex::Global(100),
                ..default()
            },
            InGameEntity,
            LoadingScreenRoot,
        ))
        .id();

    // 1. Top Header Bar
    let header = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            border_color: BorderColor(Color::srgba(0.3, 0.4, 0.5, 0.25)),
            ..default()
        })
        .with_children(|bar| {
            bar.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|title_box| {
                title_box.spawn((
                    TextBundle::from_section(
                        "OPTS TRANSIT OPERATIONS",
                        TextStyle {
                            font_size: 18.0,
                            color: theme.scheme.get(ColorRole::Primary),
                            ..default()
                        },
                    ),
                    TypeStyleToken(TypeScale::TitleMedium),
                    ForegroundRole(ColorRole::Primary),
                ));

                title_box.spawn((
                    TextBundle::from_section(
                        "Clean-Room Simulation Engine & Asset Pipeline",
                        TextStyle {
                            font_size: 12.0,
                            color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                            ..default()
                        },
                    ),
                    TypeStyleToken(TypeScale::LabelSmall),
                    ForegroundRole(ColorRole::OnSurfaceVariant),
                ));
            });

            // Stage Status Badge Chip
            bar.spawn((
                NodeBundle {
                    style: Style {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    border_color: BorderColor(theme.scheme.get(ColorRole::Primary)),
                    background_color: BackgroundColor(
                        theme.scheme.get(ColorRole::PrimaryContainer),
                    ),
                    ..default()
                },
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Full),
            ))
            .with_children(|chip| {
                chip.spawn((
                    TextBundle::from_section(
                        "STAGE 1/5: INITIALIZING",
                        TextStyle {
                            font_size: 11.0,
                            color: theme.scheme.get(ColorRole::OnPrimaryContainer),
                            ..default()
                        },
                    ),
                    StageIndicatorText,
                    TypeStyleToken(TypeScale::LabelSmall),
                    ForegroundRole(ColorRole::OnPrimaryContainer),
                ));
            });
        })
        .id();

    // 2. Center Content Card: Manifest Summary + Progress Bar + Telemetry Ticker
    let center_card = commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(85.0),
                    max_width: Val::Px(960.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Stretch,
                    row_gap: Val::Px(28.0),
                    padding: UiRect::all(Val::Px(32.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                border_color: BorderColor(Color::srgba(0.3, 0.45, 0.6, 0.35)),
                background_color: BackgroundColor(Color::srgba(0.10, 0.13, 0.18, 0.85)),
                ..default()
            },
            GlassSurface::new(ColorRole::SurfaceContainerHigh, 0.85),
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Large),
        ))
        .with_children(|card| {
            // Title & Brief
            card.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|head| {
                head.spawn((
                    TextBundle::from_section(
                        "Preparing Transit Session",
                        TextStyle {
                            font_size: 26.0,
                            color: Color::WHITE,
                            ..default()
                        },
                    ),
                    TypeStyleToken(TypeScale::HeadlineMedium),
                    ForegroundRole(ColorRole::OnSurface),
                ));

                head.spawn((
                    TextBundle::from_section(
                        "Synthesizing scenario geometry, vehicle mechanics, and environmental lighting",
                        TextStyle {
                            font_size: 13.0,
                            color: Color::srgb(0.7, 0.75, 0.82),
                            ..default()
                        },
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(ColorRole::OnSurfaceVariant),
                ));
            });

            // 3-Column Manifest Cards
            card.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(16.0),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                ..default()
            })
            .with_children(|manifest_row| {
                // Card 1: Map & Terminal
                spawn_manifest_tile(
                    manifest_row,
                    "SCENARIO MAP",
                    &setup.selected_map,
                    &setup.selected_entry_point,
                    Color::srgb(0.2, 0.6, 0.9),
                );

                // Card 2: Vehicle & Livery
                spawn_manifest_tile(
                    manifest_row,
                    "FLEET VEHICLE",
                    &setup.selected_vehicle,
                    &setup.selected_livery,
                    Color::srgb(0.9, 0.65, 0.2),
                );

                // Card 3: Environment & Clock
                let time_str = format!("{:02}:{:02} ({:04}-{:02}-{:02})", setup.hour, setup.minute, setup.year, setup.month, setup.day);
                spawn_manifest_tile(
                    manifest_row,
                    "ENVIRONMENT",
                    &setup.weather_preset,
                    &time_str,
                    Color::srgb(0.3, 0.8, 0.5),
                );
            });

            // Linear Progress Bar Section
            card.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|progress_box| {
                // Header row with Label & Percentage Text
                progress_box
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            TextBundle::from_section(
                                "ASSET COMPILATION & WORLD SYNTHESIS",
                                TextStyle {
                                    font_size: 12.0,
                                    color: Color::srgb(0.65, 0.72, 0.80),
                                    ..default()
                                },
                            ),
                            TypeStyleToken(TypeScale::LabelMedium),
                            ForegroundRole(ColorRole::Outline),
                        ));

                        row.spawn((
                            TextBundle::from_section(
                                "5%",
                                TextStyle {
                                    font_size: 15.0,
                                    color: theme.scheme.get(ColorRole::Primary),
                                    ..default()
                                },
                            ),
                            ProgressPercentText,
                            TypeStyleToken(TypeScale::TitleSmall),
                            ForegroundRole(ColorRole::Primary),
                        ));
                    });

                // Progress Bar Track
                progress_box
                    .spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                height: Val::Px(12.0),
                                border: UiRect::all(Val::Px(1.0)),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            border_color: BorderColor(Color::srgba(0.35, 0.45, 0.55, 0.5)),
                            background_color: BackgroundColor(Color::srgba(0.12, 0.15, 0.20, 0.9)),
                            ..default()
                        },
                        RadiusToken(opts_gui::theme::shape::RadiusLevel::Full),
                    ))
                    .with_children(|track| {
                        // Filled Indicator
                        track.spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(5.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                background_color: BackgroundColor(
                                    theme.scheme.get(ColorRole::Primary),
                                ),
                                ..default()
                            },
                            ProgressBarFill,
                            RadiusToken(opts_gui::theme::shape::RadiusLevel::Full),
                        ));
                    });
            });

            // Live Telemetry Status Ticker
            card.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    border_color: BorderColor(Color::srgba(0.25, 0.35, 0.45, 0.4)),
                    background_color: BackgroundColor(Color::srgba(0.06, 0.08, 0.11, 0.95)),
                    ..default()
                },
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
            ))
            .with_children(|ticker| {
                ticker.spawn((
                    TextBundle::from_section(
                        "> Initializing 3D simulation engine & coordinates...",
                        TextStyle {
                            font_size: 13.0,
                            color: Color::srgb(0.75, 0.85, 0.95),
                            ..default()
                        },
                    ),
                    StatusTickerText,
                    TypeStyleToken(TypeScale::BodySmall),
                ));
            });
        })
        .id();

    // 3. Bottom Navigation Controls
    let footer = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(85.0),
                max_width: Val::Px(960.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        })
        .with_children(|nav| {
            // Cancel Button
            nav.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    border_color: BorderColor(Color::srgba(0.4, 0.45, 0.55, 0.5)),
                    background_color: BackgroundColor(Color::srgba(0.12, 0.15, 0.20, 0.8)),
                    ..default()
                },
                M3Button::default(),
                CancelLoadingBtn,
                GlassSurface::new(ColorRole::SurfaceContainerHigh, 0.70),
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
            ))
            .with_children(|btn| {
                btn.spawn((
                    TextBundle::from_section(
                        "< Cancel & Return to Menu",
                        TextStyle {
                            font_size: 13.0,
                            color: Color::srgb(0.85, 0.88, 0.92),
                            ..default()
                        },
                    ),
                    TypeStyleToken(TypeScale::LabelMedium),
                ));
            });

            // Start Driving Button (Hidden/Disabled until complete)
            nav.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect::axes(Val::Px(28.0), Val::Px(12.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    border_color: BorderColor(theme.scheme.get(ColorRole::Primary)),
                    background_color: BackgroundColor(theme.scheme.get(ColorRole::Primary)),
                    ..default()
                },
                M3Button::default(),
                StartDrivingBtn,
                GlassSurface::new(ColorRole::Primary, 0.90),
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
            ))
            .with_children(|btn| {
                btn.spawn((
                    TextBundle::from_section(
                        "Start Driving Now ->",
                        TextStyle {
                            font_size: 14.0,
                            color: theme.scheme.get(ColorRole::OnPrimary),
                            ..default()
                        },
                    ),
                    TypeStyleToken(TypeScale::TitleSmall),
                    ForegroundRole(ColorRole::OnPrimary),
                ));
            });
        })
        .id();

    commands
        .entity(root)
        .push_children(&[header, center_card, footer]);
}

fn spawn_manifest_tile(
    parent: &mut ChildBuilder,
    category: &'static str,
    title: &str,
    subtitle: &str,
    accent: Color,
) {
    parent
        .spawn((
            NodeBundle {
                style: Style {
                    flex_grow: 1.0,
                    flex_basis: Val::Percent(30.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                border_color: BorderColor(Color::srgba(0.3, 0.4, 0.5, 0.3)),
                background_color: BackgroundColor(Color::srgba(0.08, 0.10, 0.14, 0.8)),
                ..default()
            },
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Medium),
        ))
        .with_children(|tile| {
            tile.spawn((
                TextBundle::from_section(
                    category,
                    TextStyle {
                        font_size: 10.0,
                        color: accent,
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::LabelSmall),
            ));

            tile.spawn((
                TextBundle::from_section(
                    title,
                    TextStyle {
                        font_size: 15.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::TitleSmall),
            ));

            tile.spawn((
                TextBundle::from_section(
                    subtitle,
                    TextStyle {
                        font_size: 12.0,
                        color: Color::srgb(0.68, 0.72, 0.78),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::BodySmall),
            ));
        });
}
