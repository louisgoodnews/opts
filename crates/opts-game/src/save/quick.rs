//! Quick save (F5), quick load (F9) and the autosave interval.
//!
//! The two keys act on one fixed save each, `quicksave` and `autosave` (their names come from the
//! message tables, their ids are fixed so a change of language never makes a second file). The
//! Help table lists the keys (`help::controls::GROUPS`, source file: this one).

use bevy::prelude::*;
use opts_gui::components::toast::ToastSeverity;

use super::ops::{save_session, session_running, SaveRequest};
use super::restore::{begin_restore, PendingRestore};
use crate::app_state::AppState;
use crate::pause::Pause;
use crate::settings::GameSettings;
use crate::shell::keys;
use crate::shell::notify::NotifyExt;
use opts_gui::l10n::Localization;

/// Quick save.
pub const QUICK_SAVE_KEY: KeyCode = KeyCode::F5;
/// Quick load.
pub const QUICK_LOAD_KEY: KeyCode = KeyCode::F9;
/// File id of the quick save.
pub const QUICK_ID: &str = "quicksave";
/// File id of the autosave.
pub const AUTO_ID: &str = "autosave";
/// Settings key of the autosave interval in minutes (0: off).
pub const AUTOSAVE_SETTING: &str = "gameplay.autosave_minutes";

/// Saves under a fixed id with the localised `name_key` and reports the result as a toast.
pub fn fixed_save(world: &mut World, id: &str, name_key: &'static str, ok_key: &'static str) {
    let name = world
        .get_resource::<Localization>()
        .map_or_else(|| id.to_owned(), |l| l.t(name_key));
    let result = save_session(
        world,
        &SaveRequest {
            name: name.clone(),
            id: Some(id.to_owned()),
            overwrite: true,
        },
    );
    let mut commands = world.commands();
    match result {
        Ok(_) => commands.notify(ToastSeverity::Info, ok_key, vec![("name", name)]),
        Err(e) => commands.notify(
            ToastSeverity::Error,
            keys::SAVE_FAILED_TOAST,
            vec![("error", e.to_string())],
        ),
    }
    world.flush();
}

/// F5: save to the quick slot.
pub fn quick_save(world: &mut World) {
    fixed_save(
        world,
        QUICK_ID,
        keys::SAVE_QUICK_NAME,
        keys::SAVE_SAVED_TOAST,
    );
}

/// F9: load the quick slot.
pub fn quick_load(world: &mut World) {
    if let Err(e) = begin_restore(world, QUICK_ID) {
        let mut commands = world.commands();
        commands.notify(
            ToastSeverity::Error,
            keys::LOAD_FAILED_TOAST,
            vec![("error", e.to_string())],
        );
        world.flush();
    }
}

/// F5 and F9 in a drive; ignored while a dialog is open (registered with the modal gate).
pub fn quick_keys(
    keyboard: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<opts_bevy::ActionKeys>>,
    mut commands: Commands,
) {
    let keys = opts_bevy::action_keys_of(&action_keys);
    if keys.just_pressed(&keyboard, "quick_save") {
        commands.queue(|world: &mut World| {
            if session_running(world) {
                quick_save(world);
            }
        });
    }
    if keys.just_pressed(&keyboard, "quick_load") {
        commands.queue(|world: &mut World| {
            if session_running(world) && !world.contains_resource::<PendingRestore>() {
                quick_load(world);
            }
        });
    }
}

/// Saves every N minutes of real play time when the setting is on.
pub fn autosave(
    time: Res<Time<Real>>,
    settings: Option<Res<GameSettings>>,
    pause: Res<Pause>,
    mut since: Local<f32>,
    mut commands: Commands,
) {
    let minutes: f32 = settings
        .and_then(|s| s.raw(AUTOSAVE_SETTING)?.parse().ok())
        .unwrap_or(0.0);
    if minutes < 1.0 {
        *since = 0.0;
        return;
    }
    if pause.open {
        return;
    }
    *since += time.delta_secs();
    if *since < minutes * 60.0 {
        return;
    }
    *since = 0.0;
    commands.queue(|world: &mut World| {
        if session_running(world) && !world.contains_resource::<PendingRestore>() {
            fixed_save(
                world,
                AUTO_ID,
                keys::SAVE_AUTO_NAME,
                keys::SAVE_AUTOSAVED_TOAST,
            );
        }
    });
}

/// Registers the keys and the autosave.
#[derive(Default)]
pub struct QuickSavePlugin;

impl Plugin for QuickSavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                quick_keys.run_if(opts_bevy::reporting::not_modal_active),
                autosave,
            )
                .run_if(in_state(AppState::InGame)),
        );
    }
}
