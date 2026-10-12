//! Choice buttons (quick chips, weather cards, livery buttons): one look for "this is selected" —
//! container fill plus a check mark, never colour alone.

use bevy::prelude::*;
use opts_gui::interaction::state::Selected;
use opts_gui::patterns::glyphs;
use opts_gui::prelude::*;

use super::planner::{matching_quick_time, PlannerStep};
use super::types::{GameCatalogue, NewGameSetup};
use super::ui::{glyph, Act, PlannerAction};
use super::weather::preset_of;

/// A choice button; `mark` is its check mark node.
#[derive(Component, Debug, Clone, Copy)]
pub struct Choice {
    /// The check mark.
    pub mark: Entity,
}

/// The step a choice belongs to (so only the shown step's choices are synced).
#[derive(Component, Debug, Clone, Copy)]
pub struct ChoiceStep(pub PlannerStep);

/// Spawns the empty frame of a choice button; the caller adds its content and parents it.
pub fn spawn_choice(
    commands: &mut Commands,
    action: PlannerAction,
    id: String,
    node: Node,
) -> Entity {
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
        Act(action),
        UiId::new(id),
        Choice { mark },
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

/// Whether the action's choice is the current one.
pub fn is_chosen(action: &PlannerAction, setup: &NewGameSetup, default_livery: &str) -> bool {
    match action {
        PlannerAction::Quick(i) => matching_quick_time(setup) == Some(*i),
        PlannerAction::Weather(id) => preset_of(&setup.weather_preset).id == *id,
        PlannerAction::Livery(name) => {
            let current = if setup.selected_livery.is_empty() {
                default_livery
            } else {
                setup.selected_livery.as_str()
            };
            current == name
        }
        _ => false,
    }
}

/// Applies selection look to every choice button.
pub fn sync_choices(
    setup: Res<NewGameSetup>,
    catalogue: Res<GameCatalogue>,
    choices: Query<(Entity, &Act, &Choice, Has<Selected>)>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
    added: Query<(), Added<Choice>>,
) {
    if !setup.is_changed() && added.is_empty() {
        return;
    }
    let default_livery = catalogue
        .find_vehicle(&setup.selected_vehicle)
        .map(|v| v.default_livery.clone())
        .unwrap_or_default();
    for (entity, act, choice, was) in &choices {
        let on = is_chosen(&act.0, &setup, &default_livery);
        if on != was || !added.is_empty() {
            let mut e = commands.entity(entity);
            if on {
                e.insert((Selected, BackgroundRole(ColorRole::SecondaryContainer)));
            } else {
                e.remove::<Selected>();
                e.insert(BackgroundRole(ColorRole::Surface));
            }
            if let Ok(mut n) = nodes.get_mut(choice.mark) {
                n.display = if on { Display::Flex } else { Display::None };
            }
        }
    }
}
