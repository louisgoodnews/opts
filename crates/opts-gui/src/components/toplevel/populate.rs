//! Header and body of a toplevel window: the title bar, the close button and the content node.

use crate::components::Button;
use bevy::prelude::*;

use crate::theme::{
    color::ColorRole, elevation::ElevationLevel, shape::RadiusLevel, typography::TypeScale,
};
use crate::tokens::components::{
    BackgroundRole, ElevationToken, ForegroundRole, RadiusToken, TypeStyleToken,
};

use super::{ToplevelBody, ToplevelCloseButton};

pub(super) fn spawn_window_header(
    window_parent: &mut ChildSpawnerCommands,
    root: Entity,
    title: &str,
) {
    window_parent
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|header_parent| {
            header_parent.spawn((
                (
                    Text::new(title.to_string()),
                    TextFont {
                        font_size: FontSize::Px(20.0),
                        ..default()
                    },
                    TextColor::default(),
                ),
                ForegroundRole(ColorRole::OnSurface),
                TypeStyleToken(TypeScale::TitleMedium),
            ));

            // Close Button
            header_parent
                .spawn((
                    (
                        Button,
                        Node {
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ),
                    ToplevelCloseButton(root),
                    RadiusToken(RadiusLevel::Full),
                ))
                .with_children(|close_parent| {
                    close_parent.spawn((
                        (
                            Text::new("✕"),
                            TextFont {
                                font_size: FontSize::Px(16.0),
                                ..default()
                            },
                            TextColor::default(),
                        ),
                        ForegroundRole(ColorRole::OnSurfaceVariant),
                        TypeStyleToken(TypeScale::BodyMedium),
                    ));
                });
        });
}

pub(super) fn populate_toplevel_children(
    scrim_parent: &mut ChildSpawnerCommands,
    root: Entity,
    title: &str,
    width: f32,
    height: f32,
) {
    // Window Card
    scrim_parent
        .spawn((
            (Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                width: Val::Px(width),
                min_height: Val::Px(height),
                padding: UiRect::all(Val::Px(24.0)),
                row_gap: Val::Px(16.0),
                ..default()
            }),
            BackgroundRole(ColorRole::SurfaceContainerHigh),
            RadiusToken(RadiusLevel::Large),
            ElevationToken(ElevationLevel::Level3),
        ))
        .with_children(|window_parent| {
            spawn_window_header(window_parent, root, title);

            // Inner body container for custom contents
            window_parent.spawn((
                (Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    width: Val::Percent(100.0),
                    ..default()
                }),
                ToplevelBody(root),
            ));
        });
}
