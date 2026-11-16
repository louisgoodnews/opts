//! The *Save game* and *Load game* overlay: a modal list of saves over the pause overlay (save and
//! load) or over Home (load). Rows: name, map, vehicles, play time, date, content warnings, and
//! *Load*, *Overwrite*, *Rename*, *Delete*. Destructive steps ask first (the shell's dialog);
//! failures use the error template (what happened, why, *Choose another*, *Back*).

mod dialogs;
mod rows;

use bevy::prelude::*;
use opts_gui::components::modal::{ModalBuilder, ModalEvent};
use opts_gui::components::text_field::{M3TextField, TextFieldBuilder};
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::L10nKey;
use opts_gui::prelude::*;

use super::container::SaveError;
use super::ops::{save_session, SaveRequest};
use crate::app_state::AppState;
use crate::shell::keys;

/// Backdrop of the overlay.
#[derive(Component, Debug, Clone, Copy)]
pub struct SavesOverlay;

/// What the overlay is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SavesMode {
    /// Pause overlay: save the drive (and load another).
    #[default]
    Save,
    /// Home: load.
    Load,
}

/// A control of the overlay.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub enum SavesButton {
    /// Save under the name in the field.
    Save,
    /// Apply the name in the field to the save being renamed.
    ApplyRename,
    /// Close the overlay.
    Close,
    /// A button of one row.
    Row(String, RowAction),
}

/// What a row's button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowAction {
    /// Restore this save.
    Load,
    /// Replace this save with the running drive.
    Overwrite,
    /// Change the name.
    Rename,
    /// Remove it.
    Delete,
}

/// The container of the rows.
#[derive(Component, Debug, Clone, Copy)]
pub struct SavesList;

/// The row with the name field and its buttons.
#[derive(Component, Debug, Clone, Copy)]
pub struct NameRow;

/// The name field.
#[derive(Component, Debug, Clone, Copy)]
pub struct NameField;

/// The inline error under the name field.
#[derive(Component, Debug, Clone, Copy)]
pub struct NameError;

/// State of the overlay.
#[derive(Resource, Debug, Default)]
pub struct SavesUi {
    /// What it is for.
    pub mode: SavesMode,
    /// The list must be rebuilt.
    pub dirty: bool,
    /// The save being renamed (its id).
    pub renaming: Option<String>,
    /// A question waits for its answer: what it is about.
    pub pending: Option<dialogs::Pending>,
    /// The error key shown under the name field, if any.
    pub name_error: Option<&'static str>,
}

/// Opens the overlay (a second call while it is open does nothing).
pub fn open_saves_overlay(commands: &mut Commands, mode: SavesMode) {
    commands.queue(move |world: &mut World| {
        if world
            .query_filtered::<(), With<SavesOverlay>>()
            .iter(world)
            .next()
            .is_some()
        {
            return;
        }
        *world.resource_mut::<SavesUi>() = SavesUi {
            mode,
            dirty: true,
            ..Default::default()
        };
        let texts = world
            .get_resource::<opts_gui::l10n::Localization>()
            .map(|l| {
                (
                    l.t(keys::SAVES_NAME_LABEL),
                    l.t(keys::SAVES_NAME_PLACEHOLDER),
                )
            })
            .unwrap_or_default();
        let mut commands = world.commands();
        spawn_overlay(&mut commands, mode, texts);
        world.flush();
    });
}

mod name_row;
mod overlay;
use overlay::spawn_overlay;

mod actions;
pub use actions::{close_on_cancel, handle_buttons, sync_name_row};
pub(super) use actions::{close_overlay, set_field_text};

/// Registers the overlay.
#[derive(Default)]
pub struct SaveUiPlugin;

impl Plugin for SaveUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SavesUi>().add_systems(
            Update,
            (
                handle_buttons,
                dialogs::dialog_choices,
                rows::rebuild_list,
                sync_name_row,
            )
                .chain()
                .run_if(any_with_component::<SavesOverlay>),
        );
        app.add_systems(
            Update,
            close_on_cancel.run_if(any_with_component::<SavesOverlay>),
        );
        app.add_systems(OnEnter(AppState::InGame), |mut commands: Commands| {
            commands.queue(close_overlay);
        });
    }
}
