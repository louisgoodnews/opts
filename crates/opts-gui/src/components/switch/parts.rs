//! The parts of a switch: the track and thumb styles, and the track, thumb and label nodes.

use bevy::prelude::*;

use crate::theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale};
use crate::tokens::components::{
    BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken,
};

use super::{SwitchBuilder, SwitchThumb, SwitchTrack};

impl SwitchBuilder {
    pub(super) fn track_styles(is_on: bool) -> (ColorRole, Option<ColorRole>, JustifyContent) {
        if is_on {
            (ColorRole::Primary, None, JustifyContent::FlexEnd)
        } else {
            (
                ColorRole::SurfaceContainerHighest,
                Some(ColorRole::Outline),
                JustifyContent::FlexStart,
            )
        }
    }

    pub(super) fn thumb_styles(is_on: bool) -> (ColorRole, f32) {
        if is_on {
            (ColorRole::OnPrimary, 24.0)
        } else {
            (ColorRole::Outline, 16.0)
        }
    }

    /// The track, the thumb and the optional label under the row `root`.
    pub(super) fn spawn_parts(self, parent: &mut ChildSpawnerCommands, root: Entity) {
        let (track_bg, track_border, justify) = Self::track_styles(self.is_on);
        let (thumb_bg, thumb_size) = Self::thumb_styles(self.is_on);
        let mut track_cmd = parent.spawn((
            (Node {
                width: Val::Px(52.0),
                height: Val::Px(32.0),
                padding: UiRect::all(Val::Px(4.0)),
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: justify,
                border: if track_border.is_some() {
                    UiRect::all(Val::Px(2.0))
                } else {
                    UiRect::DEFAULT
                },
                ..default()
            }),
            SwitchTrack(root),
            BackgroundRole(track_bg),
            RadiusToken(RadiusLevel::Full),
        ));

        if let Some(border_role) = track_border {
            track_cmd.insert(BorderRole(border_role));
        }

        track_cmd.with_children(|track_parent| {
            // Thumb
            track_parent.spawn((
                (Node {
                    width: Val::Px(thumb_size),
                    height: Val::Px(thumb_size),
                    ..default()
                }),
                SwitchThumb(root),
                BackgroundRole(thumb_bg),
                RadiusToken(RadiusLevel::Full),
            ));
        });

        // Optional label
        if let Some(lbl) = self.label {
            parent.spawn((
                (
                    Text::new(lbl),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor::default(),
                ),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::OnSurface),
            ));
        }
    }
}
