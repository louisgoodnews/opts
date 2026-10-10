//! UI & Themes settings tab implementation.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;
use crate::settings::GameSettings;
use crate::settings_screen::types::{ColorSwatchButton, DarkModeToggleButton, ThemePresetButton};

/// Spawns the UI & Themes tab content.
pub fn spawn_ui_tab(parent: &mut ChildBuilder, theme: &MaterialTheme, settings: &GameSettings) {
    // Section 1: Dark Mode Toggle
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(16.0),
                margin: UiRect::bottom(Val::Px(16.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                TextBundle::from_section(
                    "Theme Mode:",
                    TextStyle {
                        font_size: 15.0,
                        color: theme.scheme.get(ColorRole::OnSurface),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::TitleSmall),
                ForegroundRole(ColorRole::OnSurface),
            ));

            let mode_label = if settings.is_dark_theme {
                "Dark Mode (Active)"
            } else {
                "Light Mode (Active)"
            };

            row.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    border_color: BorderColor(theme.scheme.get(ColorRole::OutlineVariant)),
                    background_color: BackgroundColor(
                        theme.scheme.get(ColorRole::SurfaceContainerHigh),
                    ),
                    ..default()
                },
                DarkModeToggleButton,
                GlassSurface::new(ColorRole::SurfaceContainerHigh, 0.65),
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                Focusable { tab_index: 0 },
            ))
            .with_children(|btn| {
                btn.spawn((
                    TextBundle::from_section(
                        mode_label,
                        TextStyle {
                            font_size: 13.0,
                            color: theme.scheme.get(ColorRole::Primary),
                            ..default()
                        },
                    ),
                    TypeStyleToken(TypeScale::LabelLarge),
                    ForegroundRole(ColorRole::Primary),
                ));
            });
        });

    // Section 2: Preset Themes
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                margin: UiRect::bottom(Val::Px(20.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|section| {
            section.spawn((
                TextBundle::from_section(
                    "Curated Theme Presets:",
                    TextStyle {
                        font_size: 15.0,
                        color: theme.scheme.get(ColorRole::OnSurface),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::TitleSmall),
                ForegroundRole(ColorRole::OnSurface),
            ));

            section
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|buttons| {
                    let presets = [
                        ("Default", "Standard Material"),
                        ("Amber", "Amber Transit"),
                        ("Violet", "Deep Violet"),
                        ("Emerald", "Emerald Metro"),
                        ("Teal", "Teal Coast"),
                    ];

                    for (name, label) in presets {
                        let is_active = settings.active_theme == name;
                        buttons
                            .spawn((
                                ButtonBundle {
                                    style: Style {
                                        padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                                        border: UiRect::all(Val::Px(if is_active {
                                            2.0
                                        } else {
                                            1.0
                                        })),
                                        ..default()
                                    },
                                    border_color: BorderColor(if is_active {
                                        theme.scheme.get(ColorRole::Primary)
                                    } else {
                                        theme.scheme.get(ColorRole::OutlineVariant)
                                    }),
                                    background_color: BackgroundColor(
                                        theme.scheme.get(ColorRole::SurfaceContainerLow),
                                    ),
                                    ..default()
                                },
                                ThemePresetButton { preset_name: name },
                                GlassSurface::new(ColorRole::SurfaceContainerLow, 0.60),
                                RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                                Focusable { tab_index: 0 },
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    TextBundle::from_section(
                                        label,
                                        TextStyle {
                                            font_size: 13.0,
                                            color: if is_active {
                                                theme.scheme.get(ColorRole::Primary)
                                            } else {
                                                theme.scheme.get(ColorRole::OnSurface)
                                            },
                                            ..default()
                                        },
                                    ),
                                    TypeStyleToken(TypeScale::LabelMedium),
                                    ForegroundRole(ColorRole::OnSurface),
                                ));
                            });
                    }
                });
        });

    // Section 3: Per-Widget Color Customization Palette
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                margin: UiRect::bottom(Val::Px(16.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|section| {
            section.spawn((
                TextBundle::from_section(
                    "Per-Widget Colorization (Click Swatch to Apply & Persist):",
                    TextStyle {
                        font_size: 15.0,
                        color: theme.scheme.get(ColorRole::OnSurface),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::TitleSmall),
                ForegroundRole(ColorRole::OnSurface),
            ));

            spawn_swatch_row(
                section,
                theme,
                "Button Primary Accent:",
                "Button",
                "Primary",
                &[
                    ("#F59E0B", Color::srgb(0.96, 0.62, 0.04)),
                    ("#8B5CF6", Color::srgb(0.55, 0.36, 0.96)),
                    ("#10B981", Color::srgb(0.06, 0.72, 0.51)),
                    ("#0EA5E9", Color::srgb(0.05, 0.65, 0.91)),
                    ("#EF4444", Color::srgb(0.93, 0.27, 0.27)),
                ],
            );

            spawn_swatch_row(
                section,
                theme,
                "Card & Container Role:",
                "Card",
                "SecondaryContainer",
                &[
                    ("#FDE68A", Color::srgb(0.99, 0.90, 0.54)),
                    ("#DDD6FE", Color::srgb(0.87, 0.84, 0.99)),
                    ("#A7F3D0", Color::srgb(0.65, 0.95, 0.82)),
                    ("#BAE6FD", Color::srgb(0.73, 0.90, 0.99)),
                    ("#FECACA", Color::srgb(0.99, 0.79, 0.79)),
                ],
            );
        });
}

fn spawn_swatch_row(
    parent: &mut ChildBuilder,
    theme: &MaterialTheme,
    title: &'static str,
    widget: &'static str,
    role: &'static str,
    colors: &[(&'static str, Color)],
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                TextBundle::from_section(
                    title,
                    TextStyle {
                        font_size: 13.0,
                        color: theme.scheme.get(ColorRole::OnSurface),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::LabelLarge),
                ForegroundRole(ColorRole::OnSurface),
            ));

            for &(hex, color) in colors {
                row.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(24.0),
                            height: Val::Px(24.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        border_color: BorderColor(Color::WHITE.with_alpha(0.4)),
                        background_color: BackgroundColor(color),
                        ..default()
                    },
                    ColorSwatchButton {
                        widget_type: widget,
                        role_name: role,
                        hex,
                        color,
                    },
                    RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                    Focusable { tab_index: 0 },
                ));
            }
        });
}
