//! Chrome refresh and step validation for the *Plan a drive* wizard frame.

use bevy::ecs::system::SystemParam;

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

/// The summary line of each step.
fn refresh_summary(
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
fn refresh_steppers(
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

/// What the summary and the footer are worked out from.
#[derive(SystemParam)]
pub struct ChromeFacts<'w> {
    /// The maps and vehicles read.
    pub catalogue: Res<'w, GameCatalogue>,
    /// The weather catalogue.
    pub weather: Res<'w, opts_bevy::WeatherCatalogue>,
    /// The texts.
    pub loc: Res<'w, Localization>,
}

/// The nodes of the frame that follow the wizard's state.
#[derive(SystemParam)]
pub struct ChromeNodes<'w, 's> {
    /// The summary parts.
    pub summary: Query<'w, 's, (&'static SummaryPart, &'static mut Text)>,
    /// The stepper buttons.
    pub steppers: Query<'w, 's, (Entity, &'static StepperButton, Has<Selected>)>,
    /// The footer buttons.
    pub footers: Query<'w, 's, (Entity, &'static FooterButton, Has<Disabled>)>,
    /// Every node (display).
    pub nodes: Query<'w, 's, &'static mut Node>,
    /// The other texts.
    pub texts: Query<'w, 's, &'static mut Text, Without<SummaryPart>>,
}

/// Keeps summary, stepper and footer current.
pub fn refresh_chrome(
    state: Res<PlannerState>,
    setup: Res<NewGameSetup>,
    facts: ChromeFacts,
    parts: ChromeNodes,
    mut commands: Commands,
    added: Query<(), Added<StepperButton>>,
) {
    let ChromeFacts {
        catalogue,
        weather,
        loc,
    } = facts;
    let ChromeNodes {
        mut summary,
        steppers,
        footers,
        mut nodes,
        mut texts,
    } = parts;
    if !state.is_changed()
        && !setup.is_changed()
        && !catalogue.is_changed()
        && !weather.is_changed()
        && added.is_empty()
    {
        return;
    }
    refresh_summary(&mut summary, &loc, (&catalogue, &weather), &setup);
    refresh_steppers(
        &state,
        &steppers,
        (&mut nodes, &mut texts),
        &mut commands,
        !added.is_empty(),
    );
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
