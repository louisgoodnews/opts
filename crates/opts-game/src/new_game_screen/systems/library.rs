//! The bus configuration library of the wizard: save, load, duplicate, favourite and delete.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::bus_config::BusConfiguration;

use super::{Env, now_unix};

/// *Duplicate*: a copy of the configuration under the first free name.
pub(super) fn duplicate_config(env: &mut Env, id: &str, commands: &mut Commands) {
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
pub(super) fn delete_config(env: &mut Env, id: String, commands: &mut Commands) {
    if env.library.pending_delete.as_deref() == Some(id.as_str()) {
        env.library.pending_delete = None;
        env.library.store.delete(&id);
        configs_changed(env, commands);
    } else {
        env.library.pending_delete = Some(id);
        env.state.built = None;
    }
}

/// The list changed: write it, show a failure, and build the step again.
pub(super) fn configs_changed(env: &mut Env, commands: &mut Commands) {
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
pub(super) fn configs_save(env: &mut Env, commands: &mut Commands) {
    let Some(vehicle) = env.catalogue.find_vehicle(&env.setup.selected_vehicle) else {
        return;
    };
    let typed = env.library.draft_name.trim().to_string();
    let name = if typed.is_empty() {
        crate::new_game_screen::configs::default_name(vehicle, &env.setup.selected_livery)
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
pub(super) fn configs_load(env: &mut Env, id: &str, commands: &mut Commands) {
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
