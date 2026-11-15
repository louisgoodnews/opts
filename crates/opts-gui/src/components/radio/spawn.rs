//! Spawning of a radio button at the top level.

use bevy::prelude::*;

use super::{M3Radio, RadioBuilder, RadioInnerDot, RadioOuterRing};
use crate::components::Button;
use crate::interaction::{focus::Focusable, state::Disabled};
use crate::theme::{color::ColorRole, shape::RadiusLevel, spacing::SpacingScale, typography::TypeScale};
use crate::tokens::components::{BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken};

impl RadioBuilder {
    /// Spawns the radio button into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let border_role = Self::ring_border(self.is_selected);
        let dot_bg = Self::dot_bg(self.is_selected);

        let mut row_cmd = commands.spawn((
            (
                Button,
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: SpacingScale::Small.val(),
                    min_height: Val::Px(40.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ),
            M3Radio {
                is_selected: self.is_selected,
                group_id: self.group_id,
            },
            Focusable { tab_index: 0 },
        ));

        if self.disabled {
            row_cmd.insert(Disabled);
        }

        let root = row_cmd.id();

        commands.entity(root).with_children(|parent| {
            // Outer ring
            parent
                .spawn((
                    (Node {
                        width: Val::Px(20.0),
                        height: Val::Px(20.0),
                        border: UiRect::all(Val::Px(2.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    }),
                    RadioOuterRing(root),
                    BackgroundRole(ColorRole::Surface),
                    BorderRole(border_role),
                    RadiusToken(RadiusLevel::Full),
                ))
                .with_children(|ring_parent| {
                    // Inner dot
                    ring_parent.spawn((
                        (Node {
                            width: Val::Px(10.0),
                            height: Val::Px(10.0),
                            display: if self.is_selected {
                                Display::Flex
                            } else {
                                Display::None
                            },
                            ..default()
                        }),
                        RadioInnerDot(root),
                        BackgroundRole(dot_bg),
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
        });

        root
    }
}
