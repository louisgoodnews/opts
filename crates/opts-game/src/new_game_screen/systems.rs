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
use super::options::{close_options, open_options, OptionsModal};
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
    setup: ResMut<'w, NewGameSetup>,
    catalogue: Res<'w, GameCatalogue>,
    history: ResMut<'w, PlannerHistory>,
    liveries: ResMut<'w, LiveryState>,
    library: ResMut<'w, super::configs::ConfigLibrary>,
    loc: Res<'w, Localization>,
    next_state: ResMut<'w, NextState<AppState>>,
    leave: MessageWriter<'w, NavigateBack>,
    navigate: MessageWriter<'w, Navigate>,
    modals: Query<'w, 's, Entity, With<OptionsModal>>,
}

fn go(env: &mut Env, step: PlannerStep) {
    if env.state.step != step {
        env.state.step = step;
    }
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
            PlannerAction::Quick(i) => {
                let (h, m) = quick_time_of(&QUICK_TIMES[i.min(4)], wall_clock());
                env.setup.hour = h;
                env.setup.minute = m;
                env.state.time_chosen = true;
                env.state.time_source = None;
            }
            PlannerAction::Weather(id) => id.clone_into(&mut env.setup.weather_preset),
            PlannerAction::Step(field, delta) => {
                step_field(&mut env.setup, field, delta, false);
                if Field::DATE.contains(&field) {
                    env.state.date_chosen = true;
                } else {
                    env.state.time_chosen = true;
                    env.state.time_source = None;
                }
            }
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
            PlannerAction::ResetOptions => env.setup.custom_setvars.clear(),
            PlannerAction::CloseOptions => close_options(&env.modals, &mut commands),
            PlannerAction::NoVehicle => {
                env.setup.no_vehicle = true;
                env.setup.custom_setvars.clear();
                next(&mut env);
            }
            PlannerAction::ComposeFormation => {
                commands.insert_resource(ComposeFormationOnStart);
                next(&mut env);
            }
            PlannerAction::SaveConfig => configs_save(&mut env, &mut commands),
            PlannerAction::LoadConfig(id) => configs_load(&mut env, &id, &mut commands),
            PlannerAction::FavouriteConfig(id) => {
                let on = env.library.store.find(&id).is_some_and(|c| !c.favourite);
                env.library.store.set_favourite(&id, on);
                configs_changed(&mut env, &mut commands);
            }
            PlannerAction::DuplicateConfig(id) => {
                let copy = env
                    .library
                    .store
                    .find(&id)
                    .map(|c| format!("{} {}", c.name, env.loc.t("wiz-configs-copy-suffix")));
                if let Some(base) = copy {
                    let mut name = base.clone();
                    let mut n = 2;
                    while env
                        .library
                        .store
                        .duplicate(&id, &name, now_unix())
                        .is_none()
                        && n < 100
                    {
                        name = format!("{base} {n}");
                        n += 1;
                    }
                    configs_changed(&mut env, &mut commands);
                }
            }
            PlannerAction::DeleteConfig(id) => {
                if env.library.pending_delete.as_deref() == Some(id.as_str()) {
                    env.library.pending_delete = None;
                    env.library.store.delete(&id);
                    configs_changed(&mut env, &mut commands);
                } else {
                    env.library.pending_delete = Some(id);
                    env.state.built = None;
                }
            }
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
    match env.state.step.prev() {
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
    if let Some(n) = env.state.step.next() {
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
    // The wish to compose a train holds only for a rail vehicle (the player may have gone back
    // and chosen another one).
    let rail = !setup.no_vehicle
        && env
            .catalogue
            .find_vehicle(&setup.selected_vehicle)
            .is_some_and(|v| v.class == "rail");
    if !rail {
        commands.remove_resource::<ComposeFormationOnStart>();
    }
    env.next_state.set(AppState::InGame);
}

/// Set by *Compose a train* of the What step: the formation editor opens when the drive has
/// started (`crate::in_game::open_formation_on_start`), and is removed then.
#[derive(Resource, Debug, Clone, Copy)]
pub struct ComposeFormationOnStart;

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
    steps.set_if_neq(ScreenBackSteps(state.step.prev().is_some()));
}

/// `]` and `[` switch steps like tabs.
pub fn handle_tab_actions(mut actions: MessageReader<UiAction>, mut env: Env) {
    for action in actions.read() {
        match action {
            UiAction::NextTab => next(&mut env),
            UiAction::PrevTab => {
                if let Some(p) = env.state.step.prev() {
                    env.state.step = p;
                }
            }
            _ => {}
        }
    }
}

/// The start point selector changed.
pub fn handle_start_point(
    mut events: MessageReader<ComboboxSelected>,
    boxes: Query<(), With<StartPointBox>>,
    mut setup: ResMut<NewGameSetup>,
) {
    for ev in events.read() {
        if boxes.contains(ev.entity) && setup.selected_entry_point != ev.selected_value {
            setup.selected_entry_point.clone_from(&ev.selected_value);
        }
    }
}

/// Selection, stars and *Enter* of the two lists.
#[allow(clippy::too_many_arguments)]
pub fn handle_list_events(
    mut selected: MessageReader<MasterDetailSelected>,
    mut favourites: MessageReader<MasterDetailFavourite>,
    mut chosen: MessageReader<MasterDetailChosen>,
    lists: Query<(&StepList, &MasterDetailView)>,
    mut setup: ResMut<NewGameSetup>,
    catalogue: Res<GameCatalogue>,
    weather: Res<opts_bevy::WeatherCatalogue>,
    mut history: ResMut<PlannerHistory>,
    mut liveries: ResMut<LiveryState>,
    library: Res<super::configs::ConfigLibrary>,
    mut state: ResMut<PlannerState>,
    loc: Res<Localization>,
    mut eras: Local<std::collections::HashMap<String, Option<MapEra>>>,
    fixed_today: Option<Res<super::planner::Today>>,
    mut commands: Commands,
) {
    for ev in selected.read() {
        let Ok((StepList(step), view)) = lists.get(ev.root) else {
            continue;
        };
        let detail = view.parts().detail;
        commands.entity(detail).despawn_related::<Children>();
        match (step, &ev.id) {
            (PlannerStep::Where, Some(id)) => {
                let Some(map) = catalogue.maps.iter().find(|m| &m.id == id) else {
                    continue;
                };
                choose_map(&mut setup, map, &history);
                if !state.date_chosen || !state.time_chosen {
                    // What the map says about its time, read once per map: `[years]` and
                    // `[realyearoffset]` of `global.cfg`, the clock of `laststn.osn`.
                    let era = eras
                        .entry(map.id.clone())
                        .or_insert_with(|| {
                            crate::omsi_root::known_omsi_root()
                                .and_then(|root| MapEra::read(&root, &map.id))
                        })
                        .unwrap_or_default();
                    if !state.date_chosen {
                        // The map's default date: today shifted by its `[realyearoffset]`,
                        // inside its `[years]`.
                        let today = fixed_today.as_deref().map_or_else(today, |t| t.date());
                        let date = default_map_date(today, &era);
                        if super::planner::setup_date(&setup) != date {
                            set_setup_date(&mut setup, date);
                        }
                    }
                    if !state.time_chosen {
                        // The map's saved time of day, else the time now (M-0070).
                        let ((hour, minute), source) = default_start_clock(&era, wall_clock());
                        if (setup.hour, setup.minute) != (hour, minute) {
                            (setup.hour, setup.minute) = (hour, minute);
                        }
                        if state.time_source != Some(source) {
                            state.time_source = Some(source);
                        }
                    }
                }
                let ctx = BuildCtx {
                    loc: &loc,
                    catalogue: &catalogue,
                    weather: &weather,
                    setup: &setup,
                    history: &history,
                    notes: &state.notes,
                };
                fill_where_detail(&mut commands, detail, &ctx, map);
            }
            (PlannerStep::What, Some(id)) if id == NONE_ID => {
                if !setup.no_vehicle {
                    setup.no_vehicle = true;
                    setup.custom_setvars.clear();
                }
                fill_none_detail(&mut commands, detail);
            }
            (PlannerStep::What, Some(id)) => {
                if setup.no_vehicle {
                    setup.no_vehicle = false;
                }
                let Some(vehicle) = catalogue.vehicles.iter().find(|v| &v.id == id) else {
                    continue;
                };
                let key = catalogue.vehicle_key(vehicle);
                if setup.selected_vehicle != key {
                    setup.selected_vehicle = key;
                    setup.selected_livery.clear();
                    setup.custom_setvars.clear();
                }
                fill_what_detail(
                    &mut commands,
                    detail,
                    &loc,
                    &state.notes,
                    &setup,
                    &mut liveries,
                    vehicle,
                    &library,
                    &catalogue,
                );
            }
            (PlannerStep::Where, None) => {
                fill_empty_detail(&mut commands, detail, "wiz-where-empty")
            }
            (PlannerStep::What, None) => fill_empty_detail(&mut commands, detail, "wiz-what-empty"),
            _ => {}
        }
    }
    for ev in favourites.read() {
        let Ok((StepList(step), _)) = lists.get(ev.root) else {
            continue;
        };
        let vehicle = *step == PlannerStep::What;
        if let Err(e) = history.set_favourite(vehicle, &ev.id, ev.favourite) {
            tracing::warn!("cannot save favourites: {e}");
        }
    }
    for ev in chosen.read() {
        if lists.contains(ev.root) && step_invalid(state.step, &setup, &catalogue).is_none() {
            if let Some(n) = state.step.next() {
                state.step = n;
            }
        }
    }
}
/// Makes `map` the chosen one, keeping a valid start point or picking the remembered/first one.
fn choose_map(setup: &mut NewGameSetup, map: &super::types::MapItem, history: &PlannerHistory) {
    if setup.selected_map != map.friendly_name {
        setup.selected_map.clone_from(&map.friendly_name);
    }
    if map.entry_points.contains(&setup.selected_entry_point) {
        return;
    }
    let remembered = history
        .entry_point_for(&map.friendly_name)
        .filter(|e| map.entry_points.iter().any(|p| p == e))
        .map(str::to_string);
    let pick = remembered
        .or_else(|| map.entry_points.first().cloned())
        .unwrap_or_default();
    setup.selected_entry_point = pick;
}
