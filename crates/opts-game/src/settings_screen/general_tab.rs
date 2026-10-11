//! General & Storage settings tab implementation.

use bevy::prelude::*;
use opts_gui::components::Button;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;
use crate::settings::GameSettings;
use crate::settings_screen::types::{DiscoveryIntervalButton, RunDiscoveryNowButton};

/// Spawns the General & Storage settings panel.
pub fn spawn_general_tab(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    settings: &GameSettings,
) {
    parent
        .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                max_width: Val::Px(680.0),
                ..default()
            })
        .with_children(|main_col| {
            // Section 1: Storage & Persistence Architecture
            main_col
                .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    })
                .with_children(|section| {
                    section.spawn((
                        (Text::new("Storage & Persistence Architecture:"), TextFont { font_size: FontSize::Px(16.0), ..default() }, TextColor(theme.scheme.get(ColorRole::Primary))),
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
                            (Text::new(text), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant))),
                            TypeStyleToken(TypeScale::BodyMedium),
                            ForegroundRole(ColorRole::OnSurfaceVariant),
                        ));
                    }
                });

            // Section 2: Vehicle Discovery & Periodic Refresh
            main_col
                .spawn((Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(14.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    }, BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)), BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLow))))
                .with_children(|disc_box| {
                    disc_box.spawn((
                        (Text::new("Vehicle Catalogue Discovery & SQLite Sync:"), TextFont { font_size: FontSize::Px(15.0), ..default() }, TextColor(theme.scheme.get(ColorRole::Primary))),
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
                        (Text::new(format!(
                                "OMSI catalogue (SQLite): {} files · {} entities · {}",
                                files, entities, last_time_str
                            )), TextFont { font_size: FontSize::Px(12.5), ..default() }, TextColor(theme.scheme.get(ColorRole::OnSurface))),
                        TypeStyleToken(TypeScale::BodySmall),
                        ForegroundRole(ColorRole::OnSurface),
                    ));

                    disc_box.spawn((
                        (Text::new("Periodic Discovery Interval (Runs on Game Startup):"), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant))),
                        TypeStyleToken(TypeScale::LabelLarge),
                        ForegroundRole(ColorRole::OnSurfaceVariant),
                    ));

                    // Discovery Interval Frequency Buttons
                    disc_box
                        .spawn(Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(10.0),
                                flex_wrap: FlexWrap::Wrap,
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
                                        (Button, Node {
                                                padding: UiRect::axes(Val::Px(12.0), Val::Px(7.0)),
                                                border: UiRect::all(Val::Px(if is_active { 2.0 } else { 1.0 })),
                                                ..default()
                                            }, BorderColor::all(if is_active {
                                                theme.scheme.get(ColorRole::Primary)
                                            } else {
                                                theme.scheme.get(ColorRole::OutlineVariant)
                                            }), BackgroundColor(if is_active {
                                                theme.scheme.get(ColorRole::PrimaryContainer)
                                            } else {
                                                theme.scheme.get(ColorRole::SurfaceContainerHigh)
                                            })),
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
                                            (Text::new(label), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(if is_active {
                                                        theme.scheme.get(ColorRole::OnPrimaryContainer)
                                                    } else {
                                                        theme.scheme.get(ColorRole::OnSurface)
                                                    })),
                                            TypeStyleToken(TypeScale::LabelMedium),
                                        ));
                                    });
                            }
                        });

                    // Manual Discovery Trigger Button
                    disc_box
                        .spawn((
                            (Button, Node {
                                    align_self: AlignSelf::FlexStart,
                                    padding: UiRect::axes(Val::Px(18.0), Val::Px(9.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    margin: UiRect::top(Val::Px(6.0)),
                                    ..default()
                                }, BorderColor::all(theme.scheme.get(ColorRole::Primary)), BackgroundColor(theme.scheme.get(ColorRole::Primary))),
                            M3Button::default(),
                            RunDiscoveryNowButton,
                            GlassSurface::new(ColorRole::Primary, 0.90),
                            RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                            Focusable { tab_index: 0 },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                (Text::new("Run Discovery Now"), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(theme.scheme.get(ColorRole::OnPrimary))),
                                TypeStyleToken(TypeScale::LabelLarge),
                                ForegroundRole(ColorRole::OnPrimary),
                            ));
                        });
                });
        });
}
