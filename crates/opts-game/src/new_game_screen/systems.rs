//! Event handling of the wizard: buttons, `Esc`, list events, the start point selector.

use bevy::prelude::*;
use opts_gui::actions::UiAction;
use opts_gui::interaction::events::ComboboxSelected;
use opts_gui::interaction::Activated;
use opts_gui::l10n::Localization;
use opts_gui::patterns::master_detail::{
    MasterDetailChosen, MasterDetailFavourite, MasterDetailSelected,
};
use opts_gui::patterns::MasterDetailView;
use opts_gui::prelude::*;

use super::history::PlannerHistory;
use super::livery::LiveryState;
use super::options::{close_options, open_options, reset_options, OptionsModal};
use super::planner::{
    default_map_date, default_start_clock, quick_time_of, set_setup_date, step_field, today,
    validate, Field, MapEra, PlannerState, PlannerStep, QUICK_TIMES,
};
use super::screen::step_invalid;
use super::step_what::{fill_none_detail, fill_what_detail, NONE_ID};
use super::step_where::{fill_empty_detail, fill_where_detail, StartPointBox, StepList};
use super::types::{GameCatalogue, NewGameSetup};
use super::ui::{Act, PlannerAction};
use super::BuildCtx;
use crate::app_state::AppState;
use crate::bus_config::BusConfiguration;
use crate::shell::esc::{ScreenBack, ScreenBackSteps};
use crate::shell::nav::{Navigate, NavigateBack};

/// Seconds since the unix epoch (the time stamp of a drive).
fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// The wall clock's hour and minute.
fn wall_clock() -> (u32, u32) {
    use chrono::Timelike;
    let t = chrono::Local::now().time();
    (t.hour(), t.minute())
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

/// A quick time button: the clock of the preset (relative to the wall clock).
fn quick_time(env: &mut Env, i: usize) {
    let (h, m) = quick_time_of(&QUICK_TIMES[i.min(4)], wall_clock());
    env.setup.hour = h;
    env.setup.minute = m;
    env.state.time_chosen = true;
    env.state.time_source = None;
}

/// A date or time stepper: changes the field and marks the date or the time as chosen.
fn step_time(env: &mut Env, field: Field, delta: i32) {
    step_field(&mut env.setup, field, delta, false);
    if Field::DATE.contains(&field) {
        env.state.date_chosen = true;
    } else {
        env.state.time_chosen = true;
        env.state.time_source = None;
    }
}

/// *Duplicate*: a copy of the configuration under the first free name.
fn duplicate_config(env: &mut Env, id: &str, commands: &mut Commands) {
    let copy = env
        .library
        .store
        .find(id)
        .map(|c| format!("{} {}", c.name, env.loc.t("wiz-configs-copy-suffix")));
    if let Some(base) = copy {
        let mut name = base.clone();
        let mut n = 2;
        while env.library.store.duplicate(id, &name, now_unix()).is_none() && n < 100 {
            name = format!("{base} {n}");
            n += 1;
        }
        configs_changed(env, commands);
    }
}

/// *Delete*: the first press asks, the second deletes.
fn delete_config(env: &mut Env, id: String, commands: &mut Commands) {
    if env.library.pending_delete.as_deref() == Some(id.as_str()) {
        env.library.pending_delete = None;
        env.library.store.delete(&id);
        configs_changed(env, commands);
    } else {
        env.library.pending_delete = Some(id);
        env.state.built = None;
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

/// The list changed: write it, show a failure, and build the step again.
fn configs_changed(env: &mut Env, commands: &mut Commands) {
    if let Err(e) = env.library.save() {
        tracing::warn!("cannot save the bus configurations: {e}");
        ToastBuilder::new(env.loc.t_args(
            "wiz-configs-save-failed",
            &[("reason".to_string(), e.to_string().into())],
        ))
        .with_duration(6.0)
        .spawn(commands);
    }
    env.library.pending_delete = None;
    env.state.built = None;
}

/// *Save*: the plan's vehicle, repaint and options under the name in the field.
fn configs_save(env: &mut Env, commands: &mut Commands) {
    let Some(vehicle) = env.catalogue.find_vehicle(&env.setup.selected_vehicle) else {
        return;
    };
    let typed = env.library.draft_name.trim().to_string();
    let name = if typed.is_empty() {
        super::configs::default_name(vehicle, &env.setup.selected_livery)
    } else {
        typed
    };
    let config = BusConfiguration::new(
        &name,
        &vehicle.id,
        &env.setup.selected_livery,
        env.setup
            .custom_setvars
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect(),
        now_unix(),
    );
    env.library.store.save(config);
    env.library.draft_name.clear();
    ToastBuilder::new(
        env.loc
            .t_args("wiz-configs-saved", &[("name".to_string(), name.into())]),
    )
    .with_duration(4.0)
    .spawn(commands);
    configs_changed(env, commands);
}

/// *Load*: the configuration's vehicle, repaint and options go into the plan; the check against
/// the vehicle's repaints follows when they are read ([`super::configs::finish_load`]).
fn configs_load(env: &mut Env, id: &str, commands: &mut Commands) {
    let Some(config) = env.library.store.find(id).cloned() else {
        return;
    };
    let Some(vehicle) = env
        .catalogue
        .vehicles
        .iter()
        .find(|v| v.id == config.vehicle)
    else {
        ToastBuilder::new(env.loc.t_args(
            "wiz-configs-load-failed",
            &[("name".to_string(), config.name.clone().into())],
        ))
        .with_duration(6.0)
        .spawn(commands);
        return;
    };
    env.setup.no_vehicle = false;
    env.setup.selected_vehicle = env.catalogue.vehicle_key(vehicle);
    env.setup.formation.clear();
    env.setup.selected_livery.clone_from(&config.repaint);
    env.setup.custom_setvars = config.setvars.iter().cloned().collect();
    env.library.store.touch(&config.id, now_unix());
    env.library.pending_check = Some(config.id.clone());
    // The step is built again from the plan and the repaints are read for the vehicle.
    env.liveries.vehicle.clear();
    env.liveries.repaints = None;
    configs_changed(env, commands);
}

fn back(env: &mut Env) {
    match env.state.prev_step() {
        Some(prev) => env.state.step = prev,
        None => {
            env.leave.write(NavigateBack);
        }
    }
}

fn next(env: &mut Env) {
    if step_invalid(env.state.step, &env.setup, &env.catalogue).is_some() {
        return;
    }
    if let Some(n) = env.state.next_step() {
        env.state.step = n;
    }
}

fn drive(env: &mut Env, commands: &mut Commands) {
    if validate(&env.setup, &env.catalogue).is_err() {
        return;
    }
    let setup = env.setup.clone();
    if let Err(e) = env.history.record_drive(&setup, now_unix()) {
        tracing::warn!("cannot save the drive history: {e}");
        let path = env
            .history
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        ToastBuilder::new(
            env.loc
                .t_args("wiz-save-failed", &[("path".to_string(), path.into())]),
        )
        .with_duration(6.0)
        .spawn(commands);
    }
    tracing::info!(
        map = %setup.selected_map,
        vehicle = %setup.selected_vehicle,
        "starting the drive"
    );
    // A composed train holds only for the rail vehicle it was made for (the player may have gone
    // back and chosen another one: choosing clears it).
    let rail = !setup.no_vehicle
        && env
            .catalogue
            .find_vehicle(&setup.selected_vehicle)
            .is_some_and(|v| v.class == "rail");
    if !rail {
        env.setup.formation.clear();
    }
    env.next_state.set(AppState::InGame);
}

/// `Esc` / `Back` inside the wizard: the shell's one Esc mechanism (`shell::esc`) sends
/// [`ScreenBack`] while a step is left to go back to, and leaves the screen through the back stack
/// otherwise. Dialogs, open drop-downs and text fields keep the press (the shell's gate).
pub fn handle_back_action(mut requests: MessageReader<ScreenBack>, mut env: Env) {
    for _ in requests.read() {
        back(&mut env);
    }
}

/// Tells the shell whether *Back* has a step to go back to inside the wizard.
pub fn publish_back_steps(state: Res<PlannerState>, mut steps: ResMut<ScreenBackSteps>) {
    steps.set_if_neq(ScreenBackSteps(state.prev_step().is_some()));
}

/// `]` and `[` switch steps like tabs.
pub fn handle_tab_actions(mut actions: MessageReader<UiAction>, mut env: Env) {
    for action in actions.read() {
        match action {
            UiAction::NextTab => next(&mut env),
            UiAction::PrevTab => {
                if let Some(p) = env.state.prev_step() {
                    env.state.step = p;
                }
            }
            _ => {}
        }
    }
}

mod list_events;
pub use list_events::{handle_list_events, handle_start_point};
