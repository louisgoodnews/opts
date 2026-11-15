//! Spawning of a switch as a top-level entity or as a child of a parent.

use bevy::prelude::*;

use crate::components::Button;
use crate::interaction::{focus::Focusable, state::Disabled};
use crate::theme::spacing::SpacingScale;

use super::{M3Switch, SwitchBuilder};

/// The row of a switch: the focusable container of track and label.
fn row_bundle(is_on: bool) -> impl Bundle {
    (
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
        M3Switch { is_on },
        Focusable { tab_index: 0 },
    )
}

impl SwitchBuilder {
    /// Spawns the switch into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let mut row_cmd = commands.spawn(row_bundle(self.is_on));
        if self.disabled {
            row_cmd.insert(Disabled);
        }
        let root = row_cmd.id();
        commands
            .entity(root)
            .with_children(|parent| self.spawn_parts(parent, root));
        root
    }

    /// Spawns the switch as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let mut row_cmd = parent.spawn(row_bundle(self.is_on));
        if self.disabled {
            row_cmd.insert(Disabled);
        }
        let root = row_cmd.id();
        row_cmd.with_children(|child_parent| self.spawn_parts(child_parent, root));
        root
    }
}
