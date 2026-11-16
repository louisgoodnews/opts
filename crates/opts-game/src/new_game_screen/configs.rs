//! The *Configurations* section of the *What* step (M-0077): the saved bus configurations as a
//! list with *Load*, favourite, *Duplicate* and *Delete* (asks once more), and a name field with
//! *Save* for the vehicle, repaint and options of the plan.
//!
//! The list is kept in memory ([`ConfigLibrary`], read at startup); every change is written at
//! once, a few hundred bytes on a click. The control protocol (`control_configs.rs`) works on the
//! same resource, so the screen and a script see one list.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_gui::components::text_field::TextFieldBuilder;
use opts_gui::interaction::events::TextChanged;
use opts_gui::l10n::{L10nArgs, Localization};
use opts_gui::prelude::*;

use super::choice::{spawn_choice, ChoiceStep};
use super::planner::PlannerStep;
use super::repaints::VehicleRepaints;
use super::types::{GameCatalogue, NewGameSetup, VehicleItem};
use super::ui::{action_button, column, keyed, keyed_args, plain, row, Emphasis, PlannerAction};
use crate::bus_config::{BusConfiguration, BusConfigurations, Problem, StoreProblem};

mod parts;
use parts::{name_row, problem_lines, row_buttons};

mod library;
mod rows;
mod section;

pub use library::ConfigLibrary;
use rows::{config_row, problem_text};
pub use section::spawn_configs_section;

/// The name field.
#[derive(Component)]
pub struct ConfigNameField;

/// Keeps the name field's text in the library.
pub fn track_name(
    mut edited: MessageReader<TextChanged>,
    fields: Query<&ConfigNameField>,
    mut library: ResMut<ConfigLibrary>,
) {
    for t in edited.read() {
        if fields.get(t.entity).is_ok() {
            library.draft_name.clone_from(&t.text);
        }
    }
}

/// The name a configuration gets when the field is empty: the vehicle and the repaint.
#[must_use]
pub fn default_name(vehicle: &VehicleItem, repaint: &str) -> String {
    if repaint.is_empty() {
        vehicle.full_name.clone()
    } else {
        format!("{} – {repaint}", vehicle.full_name)
    }
}

/// After *Load*: when the vehicle's repaints are known, checks the loaded repaint and options
/// against them, leaves out what the vehicle no longer offers and says so.
pub fn finish_load(
    mut library: ResMut<ConfigLibrary>,
    liveries: Res<super::livery::LiveryState>,
    catalogue: Res<GameCatalogue>,
    mut setup: ResMut<NewGameSetup>,
    loc: Res<Localization>,
    mut commands: Commands,
) {
    let Some(id) = library.pending_check.clone() else {
        return;
    };
    let Some(config) = library.store.find(&id).cloned() else {
        library.pending_check = None;
        return;
    };
    let Some(vehicle) = catalogue.vehicles.iter().find(|v| v.id == config.vehicle) else {
        library.pending_check = None;
        return;
    };
    if setup.selected_vehicle != catalogue.vehicle_key(vehicle) {
        // The user chose another vehicle meanwhile.
        library.pending_check = None;
        return;
    }
    let Some(reps) = liveries
        .repaints
        .as_ref()
        .filter(|_| liveries.vehicle == vehicle.id)
    else {
        return;
    };
    library.pending_check = None;
    let checked = config.check(&catalogue, |_| reps.clone());
    setup.selected_livery.clone_from(&checked.repaint);
    setup.custom_setvars = checked.setvars.iter().cloned().collect();
    let key = if checked.problems.is_empty() {
        "wiz-configs-loaded"
    } else {
        "wiz-configs-loaded-partly"
    };
    let args = [
        ("name".to_string(), config.name.clone().into()),
        ("count".to_string(), checked.problems.len().into()),
    ];
    ToastBuilder::new(loc.t_args(key, &args))
        .with_duration(5.0)
        .spawn(&mut commands);
}
