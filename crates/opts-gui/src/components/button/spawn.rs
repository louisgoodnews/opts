//! Spawning of a button hierarchy, at the top level or as a child of another spawner.

use bevy::prelude::*;

use super::{Button, ButtonBuilder, ButtonVariant, M3Button, OnClickCallback};
use crate::interaction::{focus::Focusable, state::Disabled};
use crate::l10n::L10nKey;
use crate::theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale};
use crate::tokens::components::{
    BackgroundRole, BorderRole, ElevationToken, ForegroundRole, RadiusToken, TypeStyleToken,
};

impl ButtonBuilder {
    /// Spawns the button entity hierarchy using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let (bg_role, fg_role, elevation) = self.roles();
        let mut entity_commands = commands.spawn((
            (Button, self.style()),
            M3Button {
                variant: self.variant,
            },
            BackgroundRole(bg_role),
            RadiusToken(RadiusLevel::Full),
            ElevationToken(elevation),
            Focusable { tab_index: 0 },
        ));

        if self.variant == ButtonVariant::Outlined {
            entity_commands.insert(BorderRole(ColorRole::Outline));
        }

        if self.is_disabled {
            entity_commands.insert(Disabled);
        }

        if let Some(cb) = self.on_click {
            entity_commands.insert(OnClickCallback(cb));
        }

        let button_entity = entity_commands.id();

        if let Some(icon_handle) = self.icon {
            commands.entity(button_entity).with_children(|parent| {
                parent.spawn((
                    ImageNode::new(icon_handle),
                    Node {
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        ..default()
                    },
                ));
            });
        }

        if !self.label.is_empty() {
            let label_text = self.label;
            let key = self.label_key;
            commands.entity(button_entity).with_children(|parent| {
                let mut label = parent.spawn((
                    (
                        Text::new(label_text),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor::default(),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(fg_role),
                ));
                if let Some(key) = key {
                    label.insert(L10nKey(key));
                }
            });
        }

        button_entity
    }

    /// Spawns the button entity hierarchy as a child inside a [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let (bg_role, fg_role, elevation) = self.roles();
        let style = self.style();
        let icon_handle = self.icon;
        let label_text = self.label;
        let is_outlined = self.variant == ButtonVariant::Outlined;
        let is_disabled = self.is_disabled;
        let on_click = self.on_click;
        let label_key = self.label_key;

        let mut entity_commands = parent.spawn((
            (Button, style),
            M3Button {
                variant: self.variant,
            },
            BackgroundRole(bg_role),
            RadiusToken(RadiusLevel::Full),
            ElevationToken(elevation),
            Focusable { tab_index: 0 },
        ));

        if is_outlined {
            entity_commands.insert(BorderRole(ColorRole::Outline));
        }

        if is_disabled {
            entity_commands.insert(Disabled);
        }

        if let Some(cb) = on_click {
            entity_commands.insert(OnClickCallback(cb));
        }

        entity_commands.with_children(|child_parent| {
            if let Some(icon) = icon_handle {
                child_parent.spawn((
                    ImageNode::new(icon),
                    Node {
                        width: Val::Px(18.0),
                        height: Val::Px(18.0),
                        ..default()
                    },
                ));
            }

            if !label_text.is_empty() {
                let mut label = child_parent.spawn((
                    (
                        Text::new(label_text),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor::default(),
                    ),
                    TypeStyleToken(TypeScale::BodyMedium),
                    ForegroundRole(fg_role),
                ));
                if let Some(key) = label_key {
                    label.insert(L10nKey(key));
                }
            }
        });

        entity_commands.id()
    }
}
