//! General & Storage settings tab implementation.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;
use crate::settings::GameSettings;
use crate::settings_screen::types::{DiscoveryIntervalButton, RunDiscoveryNowButton};

/// Spawns the General & Storage settings panel.
pub fn spawn_general_tab(
    parent: &mut ChildBuilder,
    theme: &MaterialTheme,
    settings: &GameSettings,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                max_width: Val::Px(680.0),
                ..default()
            },
            ..default()
        })
        .with_children(|main_col| {
            // Section 1: Storage & Persistence Architecture
            main_col
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|section| {
                    section.spawn((
                        TextBundle::from_section(
                            "Storage & Persistence Architecture:",
                            TextStyle {
                                font_size: 16.0,
                                color: theme.scheme.get(ColorRole::Primary),
                                ..default()
                            },
                        ),
                        TypeStyleToken(TypeScale::TitleMedium),
                        ForegroundRole(ColorRole::Primary),
                    ));

                    let db_info = GameSettings::database_path()
                        .map(|p| format!("Active Database: {}", p.display()))
                        .unwrap_or_else(|| "Active Database: SQLite in-memory".to_string());

                    let toml_info = GameSettings::toml_reference_path()
                        .map(|p| format!("Baseline Reference: {} (Immutable)", p.display()))
                        .unwrap_or_else(|| "Baseline Reference: settings.toml (Immutable)".to_string());

                    let map_info = settings
                        .recent_map_path
                        .as_ref()
                        .map(|p| format!("Recent Map: {}", p.display()))
                        .unwrap_or_else(|| "Recent Map: None".to_string());

                    for text in [
                        db_info,
                        toml_info,
                        map_info,
                        "Invariant: User modifications are committed exclusively to SQLite database."
                            .to_string(),
                    ] {
                        section.spawn((
                            TextBundle::from_section(
                                text,
                                TextStyle {
                                    font_size: 13.0,
                                    color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                                    ..default()
                                },
                            ),
                            TypeStyleToken(TypeScale::BodyMedium),
                            ForegroundRole(ColorRole::OnSurfaceVariant),
                        ));
                    }
                });

            // Section 2: Vehicle Discovery & Periodic Refresh
            main_col
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(14.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                    background_color: BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLow)),
                    ..default()
                })
                .with_children(|disc_box| {
                    disc_box.spawn((
                        TextBundle::from_section(
                            "Vehicle Catalogue Discovery & SQLite Sync:",
                            TextStyle {
                                font_size: 15.0,
                                color: theme.scheme.get(ColorRole::Primary),
                                ..default()
                            },
                        ),
                        TypeStyleToken(TypeScale::TitleSmall),
                        ForegroundRole(ColorRole::Primary),
                    ));

                    let summary = crate::catalogue_service::catalogue_db_path()
                        .filter(|p| p.exists())
                        .and_then(|p| opts_catalogue::Catalogue::open(&p).ok())
                        .map(|c| (c.totals().unwrap_or((0, 0)), c.last_scan().ok().flatten()));
                    let (files, entities) = summary.map_or((0, 0), |s| s.0);
                    let last_time_str = summary
                        .and_then(|s| s.1)
                        .map(|ts| format!("Last scan timestamp: {ts} (Unix Epoch)"))
                        .unwrap_or_else(|| "Last scan timestamp: Never".to_string());

                    disc_box.spawn((
                        TextBundle::from_section(
                            format!(
                                "OMSI catalogue (SQLite): {} files · {} entities · {}",
                                files, entities, last_time_str
                            ),
                            TextStyle {
                                font_size: 12.5,
                                color: theme.scheme.get(ColorRole::OnSurface),
                                ..default()
                            },
                        ),
                        TypeStyleToken(TypeScale::BodySmall),
                        ForegroundRole(ColorRole::OnSurface),
                    ));

                    disc_box.spawn((
                        TextBundle::from_section(
                            "Periodic Discovery Interval (Runs on Game Startup):",
                            TextStyle {
                                font_size: 13.0,
                                color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                                ..default()
                            },
                        ),
                        TypeStyleToken(TypeScale::LabelLarge),
                        ForegroundRole(ColorRole::OnSurfaceVariant),
                    ));

                    // Discovery Interval Frequency Buttons
                    disc_box
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(10.0),
                                flex_wrap: FlexWrap::Wrap,
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|interval_row| {
                            let options = [
                                (0, "Every Startup"),
                                (1, "Daily (1 Day - Default)"),
                                (7, "Weekly (7 Days)"),
                                (999, "Manual Only"),
                            ];

                            for (days, label) in options {
                                let is_active = settings.discovery_interval_days == days;
                                interval_row
                                    .spawn((
                                        ButtonBundle {
                                            style: Style {
                                                padding: UiRect::axes(Val::Px(12.0), Val::Px(7.0)),
                                                border: UiRect::all(Val::Px(if is_active { 2.0 } else { 1.0 })),
                                                ..default()
                                            },
                                            border_color: BorderColor(if is_active {
                                                theme.scheme.get(ColorRole::Primary)
                                            } else {
                                                theme.scheme.get(ColorRole::OutlineVariant)
                                            }),
                                            background_color: BackgroundColor(if is_active {
                                                theme.scheme.get(ColorRole::PrimaryContainer)
                                            } else {
                                                theme.scheme.get(ColorRole::SurfaceContainerHigh)
                                            }),
                                            ..default()
                                        },
                                        M3Button::default(),
                                        DiscoveryIntervalButton { interval_days: days },
                                        GlassSurface::new(
                                            if is_active {
                                                ColorRole::PrimaryContainer
                                            } else {
                                                ColorRole::SurfaceContainerHigh
                                            },
                                            0.60,
                                        ),
                                        RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                                        Focusable { tab_index: 0 },
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn((
                                            TextBundle::from_section(
                                                label,
                                                TextStyle {
                                                    font_size: 12.0,
                                                    color: if is_active {
                                                        theme.scheme.get(ColorRole::OnPrimaryContainer)
                                                    } else {
                                                        theme.scheme.get(ColorRole::OnSurface)
                                                    },
                                                    ..default()
                                                },
                                            ),
                                            TypeStyleToken(TypeScale::LabelMedium),
                                        ));
                                    });
                            }
                        });

                    // Manual Discovery Trigger Button
                    disc_box
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    align_self: AlignSelf::FlexStart,
                                    padding: UiRect::axes(Val::Px(18.0), Val::Px(9.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    margin: UiRect::top(Val::Px(6.0)),
                                    ..default()
                                },
                                border_color: BorderColor(theme.scheme.get(ColorRole::Primary)),
                                background_color: BackgroundColor(theme.scheme.get(ColorRole::Primary)),
                                ..default()
                            },
                            M3Button::default(),
                            RunDiscoveryNowButton,
                            GlassSurface::new(ColorRole::Primary, 0.90),
                            RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                            Focusable { tab_index: 0 },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                TextBundle::from_section(
                                    "Run Discovery Now",
                                    TextStyle {
                                        font_size: 13.0,
                                        color: theme.scheme.get(ColorRole::OnPrimary),
                                        ..default()
                                    },
                                ),
                                TypeStyleToken(TypeScale::LabelLarge),
                                ForegroundRole(ColorRole::OnPrimary),
                            ));
                        });
                });
        });
}
