//! Example demonstrating Button, Label, and Icon widgets with reactive interactions.

use bevy::prelude::*;
use opts_gui::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "OPTS GUI — Buttons, Labels & Icons".into(),
                resolution: (1024.0_f32, 700.0_f32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(OptsGuiPlugin)
        .add_systems(Startup, setup_scene)
        .add_systems(Update, handle_clicks)
        .run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // Root screen layout
    let root = commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(32.0)),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(24.0),
                    ..default()
                },
                ..default()
            },
            BackgroundRole(ColorRole::Surface),
        ))
        .id();

    // 1. Headers Section
    commands.entity(root).with_children(|parent| {
        LabelBuilder::new("Material Design 3 Widgets")
            .with_style(TypeScale::HeadlineLarge)
            .with_color(ColorRole::Primary)
            .spawn_child(parent);

        LabelBuilder::new("Tier 1 Components: Buttons, Labels & Icons")
            .with_style(TypeScale::TitleMedium)
            .with_color(ColorRole::OnSurfaceVariant)
            .spawn_child(parent);
    });

    // 2. Buttons Row Section
    commands.entity(root).with_children(|parent| {
        parent
            .spawn((RowBuilder::new()
                .with_spacing(SpacingScale::Standard)
                .build(),))
            .with_children(|row| {
                ButtonBuilder::filled("Filled")
                    .on_click(|_commands, _entity| {
                        info!("Filled button clicked!");
                    })
                    .spawn_child(row);

                ButtonBuilder::elevated("Elevated")
                    .on_click(|_commands, _entity| {
                        info!("Elevated button clicked!");
                    })
                    .spawn_child(row);

                ButtonBuilder::tonal("Tonal")
                    .on_click(|_commands, _entity| {
                        info!("Tonal button clicked!");
                    })
                    .spawn_child(row);

                ButtonBuilder::outlined("Outlined")
                    .on_click(|_commands, _entity| {
                        info!("Outlined button clicked!");
                    })
                    .spawn_child(row);

                ButtonBuilder::text("Text")
                    .on_click(|_commands, _entity| {
                        info!("Text button clicked!");
                    })
                    .spawn_child(row);

                ButtonBuilder::filled("Disabled")
                    .with_disabled(true)
                    .spawn_child(row);
            });
    });

    // 3. Typography Scale Gallery
    commands.entity(root).with_children(|parent| {
        parent
            .spawn((
                StackBuilder::new()
                    .with_spacing(SpacingScale::Small)
                    .build(),
                BackgroundRole(ColorRole::SurfaceContainerLow),
                RadiusToken(RadiusLevel::Medium),
            ))
            .with_children(|card| {
                LabelBuilder::new("Typography Scale Showcase")
                    .with_style(TypeScale::TitleLarge)
                    .with_color(ColorRole::Primary)
                    .spawn_child(card);

                LabelBuilder::new("Headline Medium (28px)")
                    .with_style(TypeScale::HeadlineMedium)
                    .spawn_child(card);

                LabelBuilder::new(
                    "Body Large — standard body text for route descriptions and timetables.",
                )
                .with_style(TypeScale::BodyLarge)
                .with_color(ColorRole::OnSurface)
                .spawn_child(card);

                LabelBuilder::new("Label Small — badge or micro metadata tag")
                    .with_style(TypeScale::LabelSmall)
                    .with_color(ColorRole::OnSurfaceVariant)
                    .spawn_child(card);
            });
    });
}

fn handle_clicks(mut events: EventReader<ButtonClicked>) {
    for event in events.read() {
        info!("Received ButtonClicked event for entity {:?}", event.0);
    }
}
