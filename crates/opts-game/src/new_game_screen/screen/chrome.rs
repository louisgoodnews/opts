//! Chrome refresh and step validation for the *Plan a drive* wizard frame.

use bevy::ecs::system::SystemParam;

use super::*;

mod footer;
mod steppers;
use footer::refresh_footer;
use steppers::{refresh_steppers, refresh_summary};

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

