//! Session vehicles: the vehicles the user brought into the session, which one is active, and the
//! HUD place label of the active one (`docs/src/architecture/session-vehicles.md`, M-0068, M-0075).
//!
//! * [`registry`]: [`SessionVehicles`], the plain-data registry (a section of the later save).
//! * [`ops`]: switch, despawn, respawn and lost-vehicle handling on the `World`.
//! * This file: the plugin, the quick-switch key, the despawn confirmation and the place label.
//!
//! Spawning is the job of `in_game::omsi_world::respawn` (it needs the loaded map); it works down the
//! registry's spawn requests.

pub mod ops;
pub mod registry;

use bevy::prelude::*;
use opts_gui::l10n::{Arg, L10nArgs, Localization};

pub use ops::{OpError, PLACE_LABEL_HIDE_DISTANCE_M};
pub use registry::{
    DespawnReason, EntrySel, Lifecycle, SessionVehicle, SessionVehicleRecord, SessionVehicles,
    SpawnParams, SpawnPoint, VehicleId, SAVE_SECTION, SAVE_SECTION_VERSION,
};

use crate::app_state::AppState;
use crate::shell::dialog::{
    spawn_confirm_dialog, DialogButtonSpec, DialogButtonStyle, DialogChoice, DialogSpec, CANCEL,
};
use crate::shell::keys;

/// Tag of the despawn confirmation.
pub const DESPAWN_DIALOG: &str = "session.despawn";
const DESPAWN: &str = "despawn";

/// The key that switches to the next session vehicle (with Shift: the previous one).
pub const QUICK_SWITCH_KEY: KeyCode = KeyCode::F8;

/// The text of the HUD place label, if it is to be shown now (`None`: no label).
///
/// It is the name of the entry point the active vehicle was spawned at, shown while the vehicle
/// stands within [`PLACE_LABEL_HIDE_DISTANCE_M`] of that place. Once it moved further away or is
/// despawned the label stays hidden until the vehicle is respawned or a new one is spawned.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PlaceLabel(pub Option<String>);

/// The vehicle a despawn confirmation is open for.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PendingDespawn(pub Option<VehicleId>);

/// Computes the label from the registry.
#[must_use]
pub fn place_label(reg: &SessionVehicles) -> Option<String> {
    let rec = reg.active()?;
    (rec.is_alive() && !rec.place_dismissed && !rec.spawn_entry_name.trim().is_empty())
        .then(|| rec.spawn_entry_name.clone())
}

/// Asks the user whether to despawn a session vehicle (the shell's confirmation dialog). The
/// answer is handled by [`despawn_choices`].
pub fn ask_despawn(world: &mut World, id: VehicleId) {
    let Some(rec) = world.resource::<SessionVehicles>().get(id).cloned() else {
        return;
    };
    world.resource_mut::<PendingDespawn>().0 = Some(id);
    let spec = DialogSpec {
        tag: DESPAWN_DIALOG,
        title_key: keys::DESPAWN_TITLE,
        body_key: keys::DESPAWN_BODY,
        body_args: L10nArgs(vec![("name".into(), Arg::from(rec.name.as_str()))]),
        buttons: vec![
            DialogButtonSpec {
                choice: CANCEL,
                label_key: keys::CANCEL,
                style: DialogButtonStyle::Quiet,
            },
            DialogButtonSpec {
                choice: DESPAWN,
                label_key: keys::DESPAWN_CONFIRM,
                style: DialogButtonStyle::Destructive,
            },
        ],
        keep_open: Vec::new(),
        error: None,
    };
    let mut commands = world.commands();
    spawn_confirm_dialog(&mut commands, spec);
    world.flush();
}

/// The answer of the despawn confirmation.
pub fn despawn_choices(
    mut commands: Commands,
    mut choices: MessageReader<DialogChoice>,
    mut pending: ResMut<PendingDespawn>,
) {
    for choice in choices.read().filter(|c| c.tag == DESPAWN_DIALOG) {
        let id = pending.0.take();
        if choice.choice != DESPAWN {
            continue;
        }
        if let Some(id) = id {
            commands.queue(move |world: &mut World| {
                if let Err(e) = ops::despawn(world, id, DespawnReason::User) {
                    tracing::warn!("despawn refused: {e}");
                }
            });
        }
    }
}

/// F8 switches to the next session vehicle, Shift+F8 to the previous one.
///
/// Registered with the other gated input systems; the row of the Help table names this file.
pub fn quick_switch(keyboard: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if !keyboard.just_pressed(QUICK_SWITCH_KEY) {
        return;
    }
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let step = if shift { -1 } else { 1 };
    commands.queue(move |world: &mut World| {
        if let Err(e) = ops::switch_step(world, step) {
            tracing::debug!("quick switch: {e}");
        }
    });
}

/// Keeps the registry honest each frame: vehicles that vanished, trailer lists, the place latch.
fn maintain(world: &mut World) {
    ops::sync_lost(world);
    ops::refresh_records(world);
}

/// Publishes the place label.
fn update_place_label(reg: Res<SessionVehicles>, mut label: ResMut<PlaceLabel>) {
    let next = PlaceLabel(place_label(&reg));
    if *label != next {
        *label = next;
    }
}

fn reset_registry(
    mut reg: ResMut<SessionVehicles>,
    mut label: ResMut<PlaceLabel>,
    mut pending: ResMut<PendingDespawn>,
) {
    reg.clear();
    *label = PlaceLabel::default();
    pending.0 = None;
}

/// The one line the pause menu shows for a vehicle: name, repaint and spawn place.
#[must_use]
pub fn row_text(loc: &Localization, rec: &SessionVehicleRecord, active: bool) -> String {
    let mut parts = vec![rec.name.clone()];
    if !rec.repaint.is_empty() {
        parts.push(rec.repaint.clone());
    }
    if !rec.spawn_entry_name.is_empty() {
        parts.push(rec.spawn_entry_name.clone());
    }
    let mut text = parts.join(" · ");
    if active {
        text = format!("{} {text}", loc.t(keys::SESSION_ACTIVE_MARK));
    }
    text
}

/// Registers the registry, the label and the systems that keep them in step.
#[derive(Default)]
pub struct SessionVehiclesPlugin;

impl Plugin for SessionVehiclesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SessionVehicles>()
            .init_resource::<PlaceLabel>()
            .init_resource::<PendingDespawn>()
            .add_systems(
                Update,
                (
                    maintain,
                    update_place_label,
                    despawn_choices,
                    quick_switch.run_if(opts_bevy::reporting::not_modal_active),
                )
                    .chain()
                    .run_if(in_state(AppState::InGame)),
            )
            .add_systems(OnEnter(AppState::InGame), reset_registry)
            .add_systems(OnExit(AppState::InGame), reset_registry);
    }
}

#[cfg(test)]
mod tests;
