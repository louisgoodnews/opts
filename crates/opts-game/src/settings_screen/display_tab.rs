//! Display & Sound settings tab implementation.

use bevy::prelude::*;
use opts_gui::prelude::*;

/// Spawns the Display & Sound settings panel.
pub fn spawn_display_tab(parent: &mut ChildBuilder, theme: &MaterialTheme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                max_width: Val::Px(640.0),
                ..default()
            },
            ..default()
        })
        .with_children(|section| {
            section.spawn((
                TextBundle::from_section(
                    "Display & Render Pipeline:",
                    TextStyle {
                        font_size: 16.0,
                        color: theme.scheme.get(ColorRole::Primary),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::TitleMedium),
                ForegroundRole(ColorRole::Primary),
            ));

            section.spawn((
                TextBundle::from_section(
                    "Resolution: 1280x720 (Default) · VSync: AutoVsync · Backend: WGPU",
                    TextStyle {
                        font_size: 13.0,
                        color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));

            section.spawn((
                TextBundle::from_section(
                    "Audio Engine: Simulation spatial mixer initialized (Master Volume 100%).",
                    TextStyle {
                        font_size: 13.0,
                        color: theme.scheme.get(ColorRole::OnSurfaceVariant),
                        ..default()
                    },
                ),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));
        });
}
