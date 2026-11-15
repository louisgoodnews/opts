//! The frame of a choice chip: the outlined button, its check mark and the chosen look.

use bevy::prelude::*;
use opts_gui::patterns::glyphs;
use opts_gui::prelude::*;

use super::stepper::glyph;

/// A choice chip; `mark` is its check mark node (shown when chosen).
#[derive(Component, Debug, Clone, Copy)]
pub struct ChoiceMark {
    /// The check mark.
    pub mark: Entity,
}

/// Spawns the empty frame of a choice button (outlined, check mark first); the caller inserts its
/// action, adds its content and parents it. `node` gives the layout.
pub fn spawn_choice_frame(commands: &mut Commands, id: String, node: Node) -> Entity {
    let button = ButtonBuilder::outlined(String::new()).spawn(commands);
    let mark = commands
        .spawn((
            glyph(glyphs::CHECK, 18.0, ColorRole::OnSecondaryContainer),
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .id();
    let mut base = commands.entity(button);
    base.insert((
        UiId::new(id),
        ChoiceMark { mark },
        RadiusToken(RadiusLevel::Medium),
    ));
    base.queue(move |mut e: EntityWorldMut| {
        if let Some(mut n) = e.get_mut::<Node>() {
            n.flex_direction = node.flex_direction;
            n.align_items = node.align_items;
            n.padding = node.padding;
            n.row_gap = node.row_gap;
            n.column_gap = node.column_gap;
            n.min_height = node.min_height;
            n.width = node.width;
            n.justify_content = node.justify_content;
        }
    });
    commands.entity(button).add_child(mark);
    button
}

/// Gives a choice frame the chosen or the plain look (container fill plus check mark, never
/// colour alone).
pub fn show_chosen<F: bevy::ecs::query::QueryFilter>(
    commands: &mut Commands,
    nodes: &mut Query<&mut Node, F>,
    entity: Entity,
    mark: &ChoiceMark,
    on: bool,
) {
    let mut e = commands.entity(entity);
    if on {
        e.insert((
            opts_gui::interaction::state::Selected,
            BackgroundRole(ColorRole::SecondaryContainer),
        ));
    } else {
        e.remove::<opts_gui::interaction::state::Selected>();
        e.insert(BackgroundRole(ColorRole::Surface));
    }
    if let Ok(mut n) = nodes.get_mut(mark.mark) {
        n.display = if on { Display::Flex } else { Display::None };
    }
}
