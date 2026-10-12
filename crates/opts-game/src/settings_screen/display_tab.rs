//! Display & Sound settings tab implementation.

use bevy::prelude::*;
use opts_gui::components::Button;
use opts_gui::prelude::*;

use crate::look::Look;
use crate::menu::glass::GlassSurface;
use crate::settings::GameSettings;
use crate::settings_screen::types::{RenderChoice, RenderChoiceButton};

/// Colours of a choice button (selected or not).
pub fn choice_colors(theme: &MaterialTheme, selected: bool) -> (Color, Color) {
    if selected {
        (
            theme.scheme.get(ColorRole::Primary),
            theme.scheme.get(ColorRole::PrimaryContainer),
        )
    } else {
        (
            theme.scheme.get(ColorRole::OutlineVariant),
            theme.scheme.get(ColorRole::SurfaceContainerHigh),
        )
    }
}

/// Spawns a titled row of radio-style buttons for `choices`.
fn spawn_choice_row(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    settings: &GameSettings,
    title: &str,
    choices: &[RenderChoice],
) {
    parent.spawn((
        Text::new(title),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(theme.scheme.get(ColorRole::OnSurface)),
    ));
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(10.0),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .with_children(|row| {
            for &choice in choices {
                let selected = choice.is_selected(settings);
                let (border, bg) = choice_colors(theme, selected);
                row.spawn((
                    (
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(12.0), Val::Px(7.0)),
                            border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
                            ..default()
                        },
                        BorderColor::all(border),
                        BackgroundColor(bg),
                    ),
                    M3Button::default(),
                    RenderChoiceButton(choice),
                    GlassSurface::new(
                        if selected {
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
                        Text::new(choice.label()),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(theme.scheme.get(if selected {
                            ColorRole::OnPrimaryContainer
                        } else {
                            ColorRole::OnSurface
                        })),
                        TypeStyleToken(TypeScale::Label),
                    ));
                });
            }
        });
}

/// Spawns the Display & Sound settings panel.
pub fn spawn_display_tab(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    settings: &GameSettings,
) {
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

            let filtering: Vec<_> = opts_bevy::TextureFiltering::ALL
                .into_iter()
                .map(RenderChoice::Filtering)
                .collect();
            spawn_choice_row(
                section,
                theme,
                settings,
                "Texture filtering (applies to textures loaded afterwards):",
                &filtering,
            );
            let looks: Vec<_> = Look::ALL.into_iter().map(RenderChoice::Look).collect();
            spawn_choice_row(
                section,
                theme,
                settings,
                "Colour look (applies immediately):",
                &looks,
            );
        });
}
