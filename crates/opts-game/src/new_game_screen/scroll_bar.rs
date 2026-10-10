//! Visual scrollbar column with integrated top/bottom stepper buttons, clickable track, and floating scroll-to-top button.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::menu::glass::GlassSurface;
use crate::new_game_screen::types::{
    ScrollDownBtn, ScrollKind, ScrollToTopBtn, ScrollUpBtn, ScrollbarThumb, ScrollbarTrack,
};

/// Spawns an interactive scrollbar column with top `▲` button, clickable track with thumb, and bottom `▼` button.
pub fn spawn_scrollbar(parent: &mut ChildSpawnerCommands, theme: &MaterialTheme, kind: ScrollKind) {
    parent
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: Val::Px(2.0),
            top: Val::Px(4.0),
            bottom: Val::Px(4.0),
            width: Val::Px(18.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|col| {
            // 1. Top Stepper Button (▲)
            col.spawn((
                (
                    Button,
                    Node {
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                    BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
                ),
                M3Button::default(),
                ScrollUpBtn { kind },
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                Focusable { tab_index: 0 },
            ))
            .with_children(|btn| {
                btn.spawn((
                    (
                        Text::new("^"),
                        TextFont {
                            font_size: FontSize::Px(9.0),
                            ..default()
                        },
                        TextColor(theme.scheme.get(ColorRole::Primary)),
                    ),
                    TypeStyleToken(TypeScale::LabelSmall),
                    ForegroundRole(ColorRole::Primary),
                ));
            });

            // 2. Clickable Scrollbar Track
            col.spawn((
                (
                    Button,
                    Node {
                        flex_grow: 1.0,
                        width: Val::Px(12.0),
                        margin: UiRect::vertical(Val::Px(3.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                    BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerLowest)),
                ),
                M3Button::default(),
                ScrollbarTrack { kind },
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Full),
                Focusable { tab_index: 0 },
            ))
            .with_children(|track| {
                // Indicator Thumb inside track
                track.spawn((
                    (
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(1.0),
                            right: Val::Px(1.0),
                            top: Val::Px(0.0),
                            height: Val::Px(48.0),
                            ..default()
                        },
                        BackgroundColor(theme.scheme.get(ColorRole::Primary)),
                    ),
                    ScrollbarThumb { kind },
                    RadiusToken(opts_gui::theme::shape::RadiusLevel::Full),
                ));
            });

            // 3. Bottom Stepper Button (▼)
            col.spawn((
                (
                    Button,
                    Node {
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(theme.scheme.get(ColorRole::OutlineVariant)),
                    BackgroundColor(theme.scheme.get(ColorRole::SurfaceContainerHigh)),
                ),
                M3Button::default(),
                ScrollDownBtn { kind },
                RadiusToken(opts_gui::theme::shape::RadiusLevel::Small),
                Focusable { tab_index: 0 },
            ))
            .with_children(|btn| {
                btn.spawn((
                    (
                        Text::new("v"),
                        TextFont {
                            font_size: FontSize::Px(9.0),
                            ..default()
                        },
                        TextColor(theme.scheme.get(ColorRole::Primary)),
                    ),
                    TypeStyleToken(TypeScale::LabelSmall),
                    ForegroundRole(ColorRole::Primary),
                ));
            });
        });
}

/// Spawns a floating action button (FAB) for scrolling back to the top of the list.
pub fn spawn_scroll_to_top_button(
    parent: &mut ChildSpawnerCommands,
    theme: &MaterialTheme,
    kind: ScrollKind,
) {
    parent
        .spawn((
            (
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(28.0),
                    bottom: Val::Px(16.0),
                    width: Val::Px(36.0),
                    height: Val::Px(36.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                BorderColor::all(theme.scheme.get(ColorRole::Primary)),
                BackgroundColor(theme.scheme.get(ColorRole::PrimaryContainer)),
            ),
            M3Button::default(),
            ScrollToTopBtn { kind },
            GlassSurface::new(ColorRole::PrimaryContainer, 0.90),
            RadiusToken(opts_gui::theme::shape::RadiusLevel::Full),
            Focusable { tab_index: 0 },
        ))
        .with_children(|fab| {
            fab.spawn((
                (
                    Text::new("^"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(theme.scheme.get(ColorRole::OnPrimaryContainer)),
                ),
                TypeStyleToken(TypeScale::TitleMedium),
                ForegroundRole(ColorRole::OnPrimaryContainer),
            ));
        });
}
