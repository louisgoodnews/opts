//! Scrollable livery/repaint selection widget and setvars configuration trigger for vehicles.

use std::fs;

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;
use crate::new_game_screen::scroll_bar::{spawn_scroll_to_top_button, spawn_scrollbar};
use crate::new_game_screen::types::{
    NewGameSetup, OpenSetvarsModalBtn, RepaintCardItem, ScrollContent, ScrollKind,
    ScrollableContainer, SelectLiveryBtn, VehicleItem,
};

/// Discovers all available repaints for a vehicle by scanning .cti files and known liveries.
pub fn get_vehicle_repaints(vehicle: &VehicleItem) -> Vec<String> {
    let mut repaints = std::collections::BTreeSet::new();

    // Always include the vehicle's declared default livery
    if !vehicle.default_livery.is_empty() {
        repaints.insert(vehicle.default_livery.clone());
    }

    // Scan external OMSI 2 installation for actual .cti files
    for base_path in crate::omsi_root::known_omsi_root()
        .map(|r| r.join("Vehicles"))
        .iter()
        .map(std::path::PathBuf::as_path)
    {
        if !base_path.is_dir() {
            continue;
        }

        let rel_folder = if vehicle.id.contains("EN92")
            || vehicle.id.contains("GN92")
            || vehicle.model.contains("NL202")
            || vehicle.model.contains("NG272")
        {
            Some("MAN_NL_NG")
        } else if vehicle.id.contains("SD") || vehicle.model.contains("SD200") {
            Some("MAN_SD200")
        } else if vehicle.model.contains("SD202")
            || vehicle.id.contains("D8")
            || vehicle.id.contains("D9")
        {
            Some("MAN_SD202")
        } else {
            None
        };

        if let Some(folder) = rel_folder {
            let tex_dir = base_path.join(folder).join("Texture");
            if let Ok(entries) = fs::read_dir(&tex_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Ok(cti_files) = fs::read_dir(&path) {
                            for cti in cti_files.flatten() {
                                if cti.path().extension().and_then(|e| e.to_str()) == Some("cti") {
                                    if let Ok(content) = fs::read_to_string(cti.path()) {
                                        let lines: Vec<&str> =
                                            content.lines().map(|l| l.trim()).collect();
                                        for i in 0..lines.len() {
                                            if lines[i] == "[item]" && i + 1 < lines.len() {
                                                let repaint_name = lines[i + 1].trim();
                                                if !repaint_name.is_empty()
                                                    && !repaint_name.starts_with('[')
                                                {
                                                    repaints.insert(repaint_name.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback standard liveries if few were discovered
    if repaints.len() <= 1 {
        for l in [
            "BVG Sandgelb Traditional",
            "BVG Sonnengelb Modern",
            "BVG Corporate Standard (White)",
            "HVL Havelland Verkehrsbetriebe",
            "Bogestra Bochum-Gelsenkirchen",
            "Vestische Strassenbahnen GmbH",
            "Wiener Linien (rot-weiß)",
            "Neutral Pure White",
        ] {
            repaints.insert(l.to_string());
        }
    }

    repaints.into_iter().collect()
}

/// Spawns the scrollable repaint selection container for the vehicle detail panel.
pub fn spawn_repaint_scrollable_selection(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    veh: &VehicleItem,
    setup: &NewGameSetup,
) {
    let repaints = get_vehicle_repaints(veh);

    // Section Title row with Configure button
    parent
        .spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|header| {
            header.spawn((
                (
                    Text::new("Select Paint Livery / Repaint (Scrollable):"),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::Primary)),
                ),
                TypeStyleToken(TypeScale::TitleSmall),
                ForegroundRole(ColorRole::Primary),
            ));

            header
                .spawn((
                    (
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BorderColor::all(theme.scheme.get(ColorRole::Primary)),
                        BackgroundColor(theme.scheme.get(ColorRole::PrimaryContainer)),
                    ),
                    M3Button::default(),
                    OpenSetvarsModalBtn,
                    GlassSurface::new(ColorRole::PrimaryContainer, 0.85),
                    RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                    Focusable { tab_index: 0 },
                ))
                .with_children(|btn| {
                    btn.spawn((
                        (
                            Text::new("Configure..."),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(theme.scheme.get(ColorRole::OnPrimaryContainer)),
                        ),
                        TypeStyleToken(TypeScale::LabelMedium),
                        ForegroundRole(ColorRole::OnPrimaryContainer),
                    ));
                });
        });

    let item_height = 52.0;
    let viewport_height = 140.0;
    let total_height = repaints.len() as f32 * item_height;
    let max_scroll_y = (total_height - viewport_height).max(0.0);

    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(viewport_height),
                position_type: PositionType::Relative,
                overflow: Overflow::clip_y(),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
            BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLowest)),
        ))
        .with_children(|viewport| {
            viewport
                .spawn((
                    (Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Relative,
                        overflow: Overflow::clip_y(),
                        ..default()
                    }),
                    ScrollableContainer {
                        kind: ScrollKind::Repaints,
                        scroll_y: 0.0,
                        max_scroll_y,
                    },
                ))
                .with_children(|scroll_container| {
                    scroll_container
                        .spawn((
                            (Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0))
                                    .with_right(Val::Px(24.0)),
                                ..default()
                            }),
                            ScrollContent {
                                kind: ScrollKind::Repaints,
                            },
                        ))
                        .with_children(|content| {
                            for repaint in &repaints {
                                let is_selected = repaint == &setup.selected_livery;
                                let border_c = if is_selected {
                                    theme.scheme.get(ColorRole::Primary)
                                } else {
                                    theme.scheme.get(ColorRole::OutlineVariant)
                                };
                                let bg_c = if is_selected {
                                    theme.scheme.get(ColorRole::PrimaryContainer)
                                } else {
                                    theme.scheme.get(ColorRole::SurfaceContainerLow)
                                };

                                content
                                    .spawn((
                                        (
                                            Button,
                                            Node {
                                                width: Val::Percent(100.0),
                                                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                                flex_direction: FlexDirection::Row,
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::SpaceBetween,
                                                border: UiRect::all(Val::Px(if is_selected {
                                                    2.0
                                                } else {
                                                    1.0
                                                })),
                                                ..default()
                                            },
                                            BorderColor::all(border_c),
                                            BackgroundColor(bg_c),
                                        ),
                                        M3Button::default(),
                                        SelectLiveryBtn {
                                            livery: repaint.clone(),
                                        },
                                        RepaintCardItem {
                                            livery: repaint.clone(),
                                        },
                                        GlassSurface::new(
                                            if is_selected {
                                                ColorRole::PrimaryContainer
                                            } else {
                                                ColorRole::SurfaceContainerLow
                                            },
                                            if is_selected { 0.85 } else { 0.50 },
                                        ),
                                        RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                                        Focusable { tab_index: 0 },
                                    ))
                                    .with_children(|btn| {
                                        btn.spawn(Node {
                                            flex_direction: FlexDirection::Row,
                                            align_items: AlignItems::Center,
                                            column_gap: Val::Px(8.0),
                                            ..default()
                                        })
                                        .with_children(
                                            |row| {
                                                row.spawn((
                                                    (
                                                        Text::new(""),
                                                        TextFont {
                                                            font_size: FontSize::Px(14.0),
                                                            ..default()
                                                        },
                                                        TextColor(
                                                            theme.scheme.get(ColorRole::Primary),
                                                        ),
                                                    ),
                                                    TypeStyleToken(TypeScale::LabelMedium),
                                                ));

                                                row.spawn((
                                                    (
                                                        Text::new(repaint),
                                                        TextFont {
                                                            font_size: FontSize::Px(12.0),
                                                            ..default()
                                                        },
                                                        TextColor(if is_selected {
                                                            theme
                                                                .scheme
                                                                .get(ColorRole::OnPrimaryContainer)
                                                        } else {
                                                            theme.scheme.get(ColorRole::OnSurface)
                                                        }),
                                                    ),
                                                    TypeStyleToken(TypeScale::LabelMedium),
                                                    ForegroundRole(ColorRole::OnSurface),
                                                ));
                                            },
                                        );

                                        btn.spawn((
                                            (
                                                Text::new(if is_selected {
                                                    "● Active Livery"
                                                } else {
                                                    "Paint Scheme"
                                                }),
                                                TextFont {
                                                    font_size: FontSize::Px(11.0),
                                                    ..default()
                                                },
                                                TextColor(if is_selected {
                                                    theme.scheme.get(ColorRole::Primary)
                                                } else {
                                                    theme.scheme.get(ColorRole::Outline)
                                                }),
                                            ),
                                            TypeStyleToken(TypeScale::LabelSmall),
                                            ForegroundRole(ColorRole::Outline),
                                        ));
                                    });
                            }
                        });
                });

            spawn_scrollbar(viewport, theme, ScrollKind::Repaints);
            spawn_scroll_to_top_button(viewport, theme, ScrollKind::Repaints);
        });
}
