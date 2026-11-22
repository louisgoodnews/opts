//! The vehicle part of the Help table (M-0141, M-0142): the vehicle keys with the keys the player
//! has now, and the mouse controls (hot-spots) of the vehicle being driven.

use bevy::prelude::*;
use opts_bevy::action_keys::display;
use opts_bevy::{KeyChord, MouseTrigger, PlayerVehicle};

use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;
use crate::shell::keys;
use crate::vehicle_keys::ACTIONS;

pub mod widgets;
use widgets::{chip, heading, key_row, note};

/// The container the lists are built into; rebuilt when the settings or the vehicle change.
#[derive(Component, Debug, Clone, Copy)]
pub struct VehicleControlsList;

/// Spawns the (empty) container; [`fill_vehicle_controls`] fills it.
pub fn spawn_list(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },
        VehicleControlsList,
    ));
}

/// The names of the mouse controls of the player's vehicle, sorted and without repeats.
fn hot_spot_names(
    spots: &Query<&MouseTrigger>,
    players: &Query<Entity, With<PlayerVehicle>>,
) -> Vec<String> {
    let owners: Vec<Entity> = players.iter().collect();
    let mut names: Vec<String> = spots
        .iter()
        .filter(|s| owners.contains(&s.owner))
        .map(|s| s.trigger.clone())
        .collect();
    names.sort_unstable_by_key(|n| n.to_lowercase());
    names.dedup();
    names
}

/// The keys the player's kind of vehicle has (a tram's layout is its own), as they are set now.
fn action_rows(
    p: &mut ChildSpawnerCommands,
    settings: &GameSettings,
    registry: &SettingsRegistry,
    tram: bool,
) {
    for a in ACTIONS.iter().filter(|a| a.scope.applies(tram)) {
        let text = registry
            .text(settings, a.setting_for(tram))
            .unwrap_or_default();
        let shown = KeyChord::parse(&text)
            .ok()
            .flatten()
            .and_then(display)
            .unwrap_or_else(|| "none".to_owned());
        key_row(p, &shown, &[], a.label_key);
    }
}

/// The cockpit group: the names of the vehicle's mouse hot spots, or that it has none.
fn cockpit_spots(p: &mut ChildSpawnerCommands, names: &[String]) {
    heading(p, keys::HELP_GROUP_COCKPIT);
    if names.is_empty() {
        note(p, keys::HELP_COCKPIT_NONE);
        return;
    }
    note(p, keys::HELP_COCKPIT_NOTE);
    p.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(6.0),
        row_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|wrap| {
        for name in names {
            chip(wrap, name, Val::Auto, &[]);
        }
    });
}

/// Builds the lists the first time and again when a setting or the vehicle's controls change.
pub fn fill_vehicle_controls(
    mut commands: Commands,
    settings: Res<GameSettings>,
    registry: Option<Res<SettingsRegistry>>,
    lists: Query<(Entity, Option<&Children>), With<VehicleControlsList>>,
    added: Query<(), Added<VehicleControlsList>>,
    probe: super::VehicleProbe,
    mut shown: Local<Vec<String>>,
) {
    let super::VehicleProbe {
        spots,
        players,
        rail,
    } = probe;
    if lists.is_empty() {
        return;
    }
    let names = hot_spot_names(&spots, &players);
    if added.is_empty() && !settings.is_changed() && names == *shown {
        return;
    }
    *shown = names.clone();
    let default_registry;
    let registry = match registry.as_deref() {
        Some(r) => r,
        None => {
            default_registry = SettingsRegistry::with_builtin();
            &default_registry
        }
    };
    for (list, children) in &lists {
        for child in children.into_iter().flatten() {
            commands.entity(*child).despawn();
        }
        commands.entity(list).with_children(|p| {
            action_rows(p, &settings, registry, !rail.is_empty());
            cockpit_spots(p, &names);
        });
    }
}
