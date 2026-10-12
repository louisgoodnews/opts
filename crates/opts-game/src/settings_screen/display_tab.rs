//! Display & Sound settings tab implementation.

use bevy::prelude::*;
use opts_gui::prelude::*;

/// Spawns the Display & Sound settings panel.
pub fn spawn_display_tab(parent: &mut ChildSpawnerCommands, theme: &MaterialTheme) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(12.0),
            max_width: Val::Px(640.0),
            ..default()
        })
        .with_children(|section| {
            section.spawn((
                (
                    Text::new("Display & Render Pipeline:"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::Primary)),
                ),
                TypeStyleToken(TypeScale::TitleMedium),
                ForegroundRole(ColorRole::Primary),
            ));

            section.spawn((
                (
                    Text::new("Resolution: 1920x1080 (Default) · VSync: AutoVsync · Backend: WGPU"),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                ),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));

            section.spawn((
                (
                    Text::new(
                        "Audio Engine: Simulation spatial mixer initialized (Master Volume 100%).",
                    ),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnSurfaceVariant)),
                ),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));
        });
}
