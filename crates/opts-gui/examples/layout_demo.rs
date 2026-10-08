//! Layout Demo Example — Renders Stack, Row, and Grid primitives side by side.

use bevy::prelude::*;
use opts_gui::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "OPTS GUI — Layout Primitives Demo".into(),
                resolution: (1024.0_f32, 600.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(OptsGuiPlugin)
        .add_systems(Startup, setup_layout_demo)
        .run();
}

fn setup_layout_demo(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // Root container
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(24.0)),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(24.0),
                    justify_content: JustifyContent::SpaceEvenly,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            },
            BackgroundRole(ColorRole::Surface),
        ))
        .with_children(|root| {
            // 1. Vertical Stack container
            root.spawn((
                StackBuilder::new()
                    .with_spacing(SpacingScale::Standard)
                    .build(),
                BackgroundRole(ColorRole::SurfaceContainerLow),
                RadiusToken(RadiusLevel::Medium),
                BorderRole(ColorRole::OutlineVariant),
            ))
            .with_children(|stack| {
                stack.spawn(TextBundle::from_section(
                    "Vertical Stack",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ));
                for i in 1..=3 {
                    stack
                        .spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(160.0),
                                    height: Val::Px(40.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                ..default()
                            },
                            BackgroundRole(ColorRole::PrimaryContainer),
                            RadiusToken(RadiusLevel::Small),
                        ))
                        .with_children(|item| {
                            item.spawn(TextBundle::from_section(
                                format!("Stack Item {i}"),
                                TextStyle {
                                    font_size: 14.0,
                                    color: Color::WHITE,
                                    ..default()
                                },
                            ));
                        });
                }
            });

            // 2. Horizontal Row container
            root.spawn((
                RowBuilder::new().with_spacing(SpacingScale::Small).build(),
                BackgroundRole(ColorRole::SurfaceContainerLow),
                RadiusToken(RadiusLevel::Medium),
                BorderRole(ColorRole::OutlineVariant),
            ))
            .with_children(|row| {
                for i in 1..=3 {
                    row.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(80.0),
                                height: Val::Px(80.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ..default()
                        },
                        BackgroundRole(ColorRole::SecondaryContainer),
                        RadiusToken(RadiusLevel::Small),
                    ))
                    .with_children(|item| {
                        item.spawn(TextBundle::from_section(
                            format!("Row {i}"),
                            TextStyle {
                                font_size: 14.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        ));
                    });
                }
            });

            // 3. CSS Grid container
            root.spawn((
                GridBuilder::new()
                    .with_columns(2)
                    .with_gap(SpacingScale::Small)
                    .build(),
                BackgroundRole(ColorRole::SurfaceContainerLow),
                RadiusToken(RadiusLevel::Medium),
                BorderRole(ColorRole::OutlineVariant),
            ))
            .with_children(|grid| {
                for i in 1..=4 {
                    grid.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(70.0),
                                height: Val::Px(70.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            ..default()
                        },
                        BackgroundRole(ColorRole::TertiaryContainer),
                        RadiusToken(RadiusLevel::Small),
                    ))
                    .with_children(|cell| {
                        cell.spawn(TextBundle::from_section(
                            format!("Cell {i}"),
                            TextStyle {
                                font_size: 13.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        ));
                    });
                }
            });
        });
}
