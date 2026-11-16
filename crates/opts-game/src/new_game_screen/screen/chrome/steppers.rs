//! The summary lines and the stepper buttons of the frame: their text, their numbers and their marks.

use super::*;

/// The summary line of each step.
pub(super) fn refresh_summary(
    summary: &mut Query<(&SummaryPart, &mut Text)>,
    loc: &Localization,
    (catalogue, weather): (&GameCatalogue, &opts_bevy::WeatherCatalogue),
    setup: &NewGameSetup,
) {
    for (part, mut text) in summary {
        let new = match part.0 {
            PlannerStep::Where => where_text(setup),
            PlannerStep::What => what_text(loc, catalogue, setup),
            _ => when_text(loc, weather, setup),
        };
        if text.0 != new {
            text.0 = new;
        }
    }
}

/// The step buttons: shown or hidden, numbered, selected, and the check mark of reached steps.
pub(super) fn refresh_steppers(
    state: &PlannerState,
    steppers: &Query<(Entity, &StepperButton, Has<Selected>)>,
    (nodes, texts): (
        &mut Query<&mut Node>,
        &mut Query<&mut Text, Without<SummaryPart>>,
    ),
    commands: &mut Commands,
    added: bool,
) {
    let shown = state.shown_steps();
    for (entity, s, selected) in steppers {
        // *Trip* only when the plan has runs; the numbers count the steps shown.
        let position = shown.iter().position(|x| *x == s.step);
        if let Ok(mut n) = nodes.get_mut(entity) {
            let display = if position.is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if n.display != display {
                n.display = display;
            }
        }
        if let (Some(p), Ok(mut t)) = (position, texts.get_mut(s.number)) {
            let label = (p + 1).to_string();
            if t.0 != label {
                t.0 = label;
            }
        }
        let now = s.step == state.step;
        if now != selected || added {
            let mut e = commands.entity(entity);
            if now {
                e.insert((Selected, BackgroundRole(ColorRole::SecondaryContainer)));
            } else {
                e.remove::<Selected>();
                e.insert(BackgroundRole(ColorRole::Surface));
            }
        }
        let reached = s.step.index() < state.step.index();
        if let Ok(mut n) = nodes.get_mut(s.check) {
            n.display = if reached {
                Display::Flex
            } else {
                Display::None
            };
        }
        if let Ok(mut n) = nodes.get_mut(s.number) {
            n.display = if reached {
                Display::None
            } else {
                Display::Flex
            };
        }
    }
}
