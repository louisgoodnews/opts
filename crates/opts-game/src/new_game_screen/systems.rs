//! Event handling of the wizard: buttons, `Esc`, list events, the start point selector.

mod library;
mod navigate;
mod time;

use bevy::prelude::*;
use opts_gui::interaction::Activated;
use opts_gui::interaction::events::ComboboxSelected;
use opts_gui::l10n::Localization;
use opts_gui::patterns::MasterDetailView;
use opts_gui::patterns::master_detail::{
    MasterDetailChosen, MasterDetailFavourite, MasterDetailSelected,
};

use super::BuildCtx;
use super::history::PlannerHistory;
use super::livery::LiveryState;
use super::options::{OptionsModal, close_options, open_options, reset_options};
use super::planner::{
    MapEra, PlannerState, PlannerStep, default_map_date, default_start_clock, set_setup_date,
    today, validate,
};
use super::screen::step_invalid;
use super::step_what::{NONE_ID, fill_none_detail, fill_what_detail};
use super::step_where::{StartPointBox, StepList, fill_empty_detail, fill_where_detail};
use super::types::{GameCatalogue, NewGameSetup};
use super::ui::{Act, PlannerAction};
use crate::app_state::AppState;
use crate::shell::nav::{Navigate, NavigateBack};

use library::{configs_changed, configs_load, configs_save, delete_config, duplicate_config};
use navigate::{back, drive, next};
use time::{quick_time, step_time, wall_clock};

pub use navigate::{handle_back_action, handle_tab_actions, publish_back_steps};

/// Seconds since the unix epoch (the time stamp of a drive).
fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Parameters shared by the action handlers.
#[derive(bevy::ecs::system::SystemParam)]
pub struct Env<'w, 's> {
    state: ResMut<'w, PlannerState>,
    pub(super) setup: ResMut<'w, NewGameSetup>,
    pub(super) catalogue: Res<'w, GameCatalogue>,
    history: ResMut<'w, PlannerHistory>,
    pub(super) liveries: ResMut<'w, LiveryState>,
    library: ResMut<'w, super::configs::ConfigLibrary>,
    pub(super) loc: Res<'w, Localization>,
    next_state: ResMut<'w, NextState<AppState>>,
    leave: MessageWriter<'w, NavigateBack>,
    navigate: MessageWriter<'w, Navigate>,
    pub(super) modals: Query<'w, 's, Entity, With<OptionsModal>>,
    formation_modals: Query<'w, 's, Entity, With<super::formation::FormationModal>>,
}

fn go(env: &mut Env, step: PlannerStep) {
    if env.state.step != step {
        env.state.step = step;
    }
}

/// *Compose a train*: makes sure the head car is set and opens the formation editor.
fn compose_formation(env: &mut Env, commands: &mut Commands) {
    super::formation::ensure_head(&mut env.setup, &env.catalogue);
    super::formation::reopen(
        commands,
        &env.formation_modals,
        &env.loc,
        &env.catalogue,
        &env.setup,
    );
}

/// Applies the action of an activated wizard button.
pub fn handle_actions(
    mut activated: MessageReader<Activated>,
    acts: Query<&Act>,
    mut env: Env,
    mut commands: Commands,
) {
    for Activated(e) in activated.read() {
        let Ok(Act(action)) = acts.get(*e) else {
            continue;
        };
        match action.clone() {
            PlannerAction::Back => back(&mut env),
            PlannerAction::Next => next(&mut env),
            PlannerAction::Skip => {
                if validate(&env.setup, &env.catalogue).is_ok() {
                    go(&mut env, PlannerStep::Review);
                }
            }
            PlannerAction::Drive => drive(&mut env, &mut commands),
            PlannerAction::GoStep(s) => go(&mut env, s),
            PlannerAction::Quick(i) => quick_time(&mut env, i),
            PlannerAction::Weather(id) => id.clone_into(&mut env.setup.weather_preset),
            PlannerAction::Step(field, delta) => step_time(&mut env, field, delta),
            PlannerAction::Livery(name) => env.setup.selected_livery = name,
            PlannerAction::OpenOptions => open_options(
                &mut commands,
                &env.loc,
                &env.catalogue,
                &env.setup,
                &env.liveries,
            ),
            PlannerAction::OpenMapConfig => commands.queue(|w: &mut World| {
                let mut menu = w.resource_mut::<crate::map_profiles::modal::MapConfigMenu>();
                menu.open = true;
                menu.selected = None;
                menu.status.clear();
            }),
            PlannerAction::ResetOptions => reset_options(&mut env, &mut commands),
            PlannerAction::CloseOptions => close_options(&env.modals, &mut commands),
            PlannerAction::NoVehicle => {
                env.setup.no_vehicle = true;
                env.setup.custom_setvars.clear();
                next(&mut env);
            }
            PlannerAction::ComposeFormation => compose_formation(&mut env, &mut commands),
            action @ (PlannerAction::FormationAdd(_)
            | PlannerAction::FormationRemove(_)
            | PlannerAction::FormationUp(_)
            | PlannerAction::FormationTurn(_)
            | PlannerAction::FormationClear
            | PlannerAction::CloseFormation) => super::formation::edit(
                action,
                &mut env.setup,
                &env.catalogue,
                &env.loc,
                &env.formation_modals,
                &mut commands,
            ),
            PlannerAction::SaveConfig => configs_save(&mut env, &mut commands),
            PlannerAction::LoadConfig(id) => configs_load(&mut env, &id, &mut commands),
            PlannerAction::FavouriteConfig(id) => {
                let on = env.library.store.find(&id).is_some_and(|c| !c.favourite);
                env.library.store.set_favourite(&id, on);
                configs_changed(&mut env, &mut commands);
            }
            PlannerAction::DuplicateConfig(id) => duplicate_config(&mut env, &id, &mut commands),
            PlannerAction::DeleteConfig(id) => delete_config(&mut env, id, &mut commands),
            PlannerAction::OpenSettings => {
                // A step into a sub-page: Back returns here.
                env.navigate.write(Navigate::push("settings"));
            }
        }
    }
}

mod list_events;
pub use list_events::{handle_list_events, handle_start_point};
