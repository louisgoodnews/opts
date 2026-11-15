//! Keeps the *When* step's stepper values, time-source hints and era hint current.

use bevy::prelude::*;
use opts_gui::l10n::L10nArgs;

use super::{EraHint, TimeSourceLabel, WhenValue};
use crate::new_game_screen::TextsAndCatalogue;
use crate::new_game_screen::planner::{Field, PlannerState, era_hint, month_key};
use crate::new_game_screen::types::NewGameSetup;

/// Parts of the step that were just spawned and need their first refresh.
type NewWhenParts = Or<(Added<WhenValue>, Added<TimeSourceLabel>)>;

/// Keeps the stepper values and the era hint current.
pub fn refresh_when(
    setup: Res<NewGameSetup>,
    state: Res<PlannerState>,
    lookup: TextsAndCatalogue,
    mut values: Query<(&WhenValue, &mut Text)>,
    mut hints: Query<(&mut Node, &mut L10nArgs), With<EraHint>>,
    mut sources: Query<(&TimeSourceLabel, &mut Node), Without<EraHint>>,
    added: Query<(), NewWhenParts>,
) {
    let TextsAndCatalogue { loc, catalogue } = lookup;
    if !setup.is_changed() && !state.is_changed() && added.is_empty() {
        return;
    }
    for (label, mut node) in &mut sources {
        let shown = if state.time_source == Some(label.0) && !state.time_chosen {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != shown {
            node.display = shown;
        }
    }
    for (v, mut text) in &mut values {
        let new = match v.0 {
            Field::Day => setup.day.to_string(),
            Field::Month => loc.t(month_key(setup.month)),
            Field::Year => setup.year.to_string(),
            Field::Hour => format!("{:02}", setup.hour),
            Field::Minute => format!("{:02}", setup.minute),
        };
        if text.0 != new {
            text.0 = new;
        }
    }
    let hint = catalogue
        .map_by_name(&setup.selected_map)
        .and_then(|m| era_hint(&m.years, setup.year));
    for (mut node, mut args) in &mut hints {
        node.display = if hint.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some((from, to)) = hint {
            *args = L10nArgs(vec![("from".into(), from.into()), ("to".into(), to.into())]);
        }
    }
}
