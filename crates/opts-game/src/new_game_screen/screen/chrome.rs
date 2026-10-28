//! Chrome refresh and step validation for the *Plan a drive* wizard frame.

use super::*;

/// The reason the current step cannot go on, if any.
pub fn step_invalid(
    step: PlannerStep,
    setup: &NewGameSetup,
    catalogue: &GameCatalogue,
) -> Option<Invalid> {
    let all = validate(setup, catalogue).err()?;
    match (step, all) {
        (PlannerStep::Where, Invalid::NoMap | Invalid::NoStartPoint) => Some(all),
        (PlannerStep::What, Invalid::NoVehicle) => Some(all),
        _ => None,
    }
}

/// Keeps summary, stepper and footer current.
#[allow(clippy::too_many_arguments)]
pub fn refresh_chrome(
    state: Res<PlannerState>,
    setup: Res<NewGameSetup>,
    catalogue: Res<GameCatalogue>,
    weather: Res<opts_bevy::WeatherCatalogue>,
    loc: Res<Localization>,
    mut summary: Query<(&SummaryPart, &mut Text)>,
    steppers: Query<(Entity, &StepperButton, Has<Selected>)>,
    footers: Query<(Entity, &FooterButton, Has<Disabled>)>,
    mut nodes: Query<&mut Node>,
    mut texts: Query<&mut Text, Without<SummaryPart>>,
    mut commands: Commands,
    added: Query<(), Added<StepperButton>>,
) {
    if !state.is_changed()
        && !setup.is_changed()
        && !catalogue.is_changed()
        && !weather.is_changed()
        && added.is_empty()
    {
        return;
    }
    for (part, mut text) in &mut summary {
        let new = match part.0 {
            PlannerStep::Where => where_text(&setup),
            PlannerStep::What => what_text(&loc, &catalogue, &setup),
            _ => when_text(&loc, &weather, &setup),
        };
        if text.0 != new {
            text.0 = new;
        }
    }
    let shown = state.shown_steps();
    for (entity, s, selected) in &steppers {
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
        if now != selected || !added.is_empty() {
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
    refresh_footer(
        &state,
        &setup,
        &catalogue,
        &loc,
        &footers,
        &mut nodes,
        &mut commands,
    );
}

fn reason_for(loc: &Localization, reason: Invalid) -> String {
    loc.t(super::super::step_review::reason_key(reason))
}

fn refresh_footer(
    state: &PlannerState,
    setup: &NewGameSetup,
    catalogue: &GameCatalogue,
    loc: &Localization,
    footers: &Query<(Entity, &FooterButton, Has<Disabled>)>,
    nodes: &mut Query<&mut Node>,
    commands: &mut Commands,
) {
    let full = validate(setup, catalogue);
    let here = step_invalid(state.step, setup, catalogue);
    let review = state.step == PlannerStep::Review;
    for (entity, kind, disabled) in footers {
        let (show, block) = match kind {
            FooterButton::Back => (true, None),
            FooterButton::Next => (!review, here),
            FooterButton::Skip => (!review, full.err()),
            FooterButton::Drive => (review, full.err()),
        };
        if let Ok(mut n) = nodes.get_mut(entity) {
            n.display = if show { Display::Flex } else { Display::None };
        }
        match (block, disabled) {
            (Some(reason), _) => {
                commands
                    .entity(entity)
                    .insert((Disabled, DisabledReason::new(reason_for(loc, reason))));
            }
            (None, true) => {
                commands
                    .entity(entity)
                    .remove::<(Disabled, DisabledReason)>();
            }
            _ => {}
        }
    }
}
