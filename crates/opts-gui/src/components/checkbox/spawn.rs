//! Spawning of a checkbox, at the top level or as a child of another spawner.

use bevy::prelude::*;

use super::{CheckboxBuilder, CheckboxGlyph, CheckboxIndicator, M3Checkbox};
use crate::components::Button;
use crate::interaction::{focus::Focusable, state::Disabled};
use crate::theme::{color::ColorRole, shape::RadiusLevel, spacing::SpacingScale, typography::TypeScale};
use crate::tokens::components::{BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken};

impl CheckboxBuilder {
    /// Spawns the checkbox into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let (bg_role, border_role, fg_role) = Self::indicator_roles(self.state);
        let glyph_str = Self::glyph_char(self.state);

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
            M3Checkbox { state: self.state },
            Focusable { tab_index: 0 },
        ));

        if self.disabled {
            row_cmd.insert(Disabled);
        }

        let root = row_cmd.id();

        commands.entity(root).with_children(|parent| {
            // Box indicator
            parent
                .spawn((
                    (Node {
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        border: UiRect::all(Val::Px(2.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    }),
                    CheckboxIndicator(root),
                    BackgroundRole(bg_role),
                    BorderRole(border_role),
                    RadiusToken(RadiusLevel::ExtraSmall),
                ))
                .with_children(|box_parent| {
                    box_parent.spawn((
                        (
                            Text::new(glyph_str),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor::default(),
                        ),
                        CheckboxGlyph(root),
                        crate::ui_tree::A11yHidden,
                        ForegroundRole(fg_role),
                        TypeStyleToken(TypeScale::Label),
                    ));
                });

            // Optional label text
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
