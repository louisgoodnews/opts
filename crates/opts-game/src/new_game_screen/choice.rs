//! Choice buttons (quick chips, weather cards, livery buttons): one look for "this is selected" —
//! container fill plus a check mark, never colour alone.

use bevy::prelude::*;
use opts_gui::interaction::state::Selected;

use super::planner::{matching_quick_time, PlannerStep};
use super::types::{GameCatalogue, NewGameSetup};
use super::ui::{Act, PlannerAction};
use opts_bevy::WeatherCatalogue;

/// A choice button and its check mark (`shell::calendar`).
pub use crate::shell::calendar::ChoiceMark as Choice;

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
    let button = crate::shell::calendar::spawn_choice_frame(commands, id, node);
    commands.entity(button).insert(Act(action));
    button
}

/// Whether the action's choice is the current one.
pub fn is_chosen(
    action: &PlannerAction,
    setup: &NewGameSetup,
    default_livery: &str,
    weather: &WeatherCatalogue,
) -> bool {
    match action {
        PlannerAction::Quick(i) => matching_quick_time(setup) == Some(*i),
        PlannerAction::Weather(id) => weather.canonical_id(&setup.weather_preset) == *id,
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
    weather: Res<WeatherCatalogue>,
    choices: Query<(Entity, &Act, &Choice, Has<Selected>)>,
    mut nodes: Query<&mut Node>,
    mut commands: Commands,
    added: Query<(), Added<Choice>>,
) {
    if !setup.is_changed() && !weather.is_changed() && added.is_empty() {
        return;
    }
    let default_livery = catalogue
        .find_vehicle(&setup.selected_vehicle)
        .map(|v| v.default_livery.clone())
        .unwrap_or_default();
    for (entity, act, choice, was) in &choices {
        let on = is_chosen(&act.0, &setup, &default_livery, &weather);
        if on != was || !added.is_empty() {
            crate::shell::calendar::show_chosen(&mut commands, &mut nodes, entity, choice, on);
        }
    }
}
