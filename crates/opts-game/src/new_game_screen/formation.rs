//! The train composer of the wizard (*Compose a train*): a dialog over the What step in which cars
//! of the installation are added behind the chosen rail vehicle, turned, moved and removed. The
//! train is kept in [`NewGameSetup::formation`] and built when the drive starts
//! (`apply_to_start`).

use bevy::prelude::*;
use opts_gui::components::button::ButtonBuilder;
use opts_gui::l10n::Localization;
use opts_gui::prelude::*;

use super::types::{GameCatalogue, NewGameSetup, VehicleItem};
use super::ui::{action_button, column, keyed, plain, row, Act, Emphasis, PlannerAction};

mod car_list;
mod train;

use car_list::{spawn_car_list, spawn_footer};
use train::spawn_train;

/// Marks the backdrop of the composer.
#[derive(Component, Debug, Clone, Copy)]
pub struct FormationModal;

/// The chosen vehicle, as the first car, when no train has been started yet.
pub fn ensure_head(setup: &mut NewGameSetup, catalogue: &GameCatalogue) {
    if !setup.formation.is_empty() {
        return;
    }
    if let Some(v) = catalogue.find_vehicle(&setup.selected_vehicle) {
        if !v.bus_file.is_empty() {
            setup.formation.push((v.bus_file.clone(), false));
        }
    }
}

/// Closes the composer.
pub fn close(modals: &Query<Entity, With<FormationModal>>, commands: &mut Commands) {
    for e in modals {
        commands.entity(e).despawn();
    }
}

/// Applies a composer action to the train and shows the result (or closes the composer).
pub fn edit(
    action: PlannerAction,
    setup: &mut NewGameSetup,
    catalogue: &GameCatalogue,
    loc: &Localization,
    modals: &Query<Entity, With<FormationModal>>,
    commands: &mut Commands,
) {
    match action {
        PlannerAction::FormationAdd(file) => setup.formation.push((file, false)),
        PlannerAction::FormationRemove(n) if n > 0 && n < setup.formation.len() => {
            setup.formation.remove(n);
        }
        PlannerAction::FormationUp(n) if n > 1 && n < setup.formation.len() => {
            setup.formation.swap(n, n - 1);
        }
        PlannerAction::FormationTurn(n) => {
            if let Some(car) = setup.formation.get_mut(n) {
                car.1 = !car.1;
            }
        }
        PlannerAction::FormationClear => setup.formation.truncate(1),
        PlannerAction::CloseFormation => {
            close(modals, commands);
            // One car is no train.
            if setup.formation.len() < 2 {
                setup.formation.clear();
            }
            return;
        }
        _ => {}
    }
    reopen(commands, modals, loc, catalogue, setup);
}

/// Builds the composer again from the setup (after every change).
pub fn reopen(
    commands: &mut Commands,
    modals: &Query<Entity, With<FormationModal>>,
    loc: &Localization,
    catalogue: &GameCatalogue,
    setup: &NewGameSetup,
) {
    close(modals, commands);
    open(commands, loc, catalogue, setup);
}

/// Opens the composer.
pub fn open(
    commands: &mut Commands,
    loc: &Localization,
    catalogue: &GameCatalogue,
    setup: &NewGameSetup,
) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(60.0, 80.0)
        .with_width_range(520.0, 820.0)
        .spawn(commands);
    commands
        .entity(root)
        .insert((FormationModal, UiId::new("wizard.what.formation.modal")));
    let title = commands
        .spawn(keyed(
            "wiz-formation-title",
            TypeScale::TitleLarge,
            ColorRole::OnSurface,
        ))
        .id();
    let hint = commands
        .spawn(keyed(
            "wiz-formation-hint",
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    let train = spawn_train(commands, loc, catalogue, setup);
    let (add_title, list) = spawn_car_list(commands, catalogue);
    let footer = spawn_footer(commands);
    commands
        .entity(content)
        .add_children(&[title, hint, train, add_title, list, footer]);
}

/// Sets the cars behind the head for the drive that starts, and returns the train to record with
/// the vehicle (`None` for a vehicle alone or a train that does not resolve).
pub fn apply_to_start(
    setup: &NewGameSetup,
    job: &mut crate::in_game::omsi_world::WorldLoadJob,
) -> Option<opts_core::formats::trains::TrainFormation> {
    tracing::debug!(cars = setup.formation.len(), "train composed in the wizard");
    if setup.formation.len() < 2 {
        return None;
    }
    let mut train = opts_core::formats::trains::TrainFormation::custom("Formation");
    for (file, turned) in &setup.formation {
        let at = train.add_file(file);
        if *turned {
            train.turn_car(at);
        }
    }
    match crate::formation::resolve(&train, job) {
        Ok(resolved) => {
            job.set_formation_cars(resolved.cars);
            train.name = train.suggested_name();
            Some(train)
        }
        Err(issues) => {
            let text: Vec<String> = issues.iter().map(ToString::to_string).collect();
            tracing::warn!("the composed train cannot be started: {}", text.join("; "));
            None
        }
    }
}
