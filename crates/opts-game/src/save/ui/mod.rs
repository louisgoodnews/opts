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
        let mut commands = world.commands();
        spawn_overlay(&mut commands, mode);
        world.flush();
    });
}

fn label(
    card: &mut ChildSpawnerCommands,
    key: &'static str,
    scale: TypeScale,
    role: ColorRole,
) -> Entity {
    card.spawn((
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(scale),
        ForegroundRole(role),
    ))
    .id()
}

fn spawn_overlay(commands: &mut Commands, mode: SavesMode) {
    let (root, card) = ModalBuilder::new()
        .with_size_percent(80.0, 85.0)
        .with_width_range(560.0, 900.0)
        .spawn(commands);
    commands
        .entity(root)
        .insert((SavesOverlay, Name::new("SavesOverlay")));
    commands.entity(card).with_children(|c| {
        let title = match mode {
            SavesMode::Save => keys::SAVES_TITLE_SAVE,
            SavesMode::Load => keys::SAVES_TITLE_LOAD,
        };
        label(c, title, TypeScale::TitleLarge, ColorRole::OnSurface);
        label(
            c,
            keys::SAVES_NOTE,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        );
        let name_row = c
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    flex_wrap: FlexWrap::Wrap,
                    display: if mode == SavesMode::Save {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    ..default()
                },
                NameRow,
            ))
            .id();
        c.commands().entity(name_row).with_children(|r| {
            let field = TextFieldBuilder::outlined(keys::SAVES_NAME_LABEL)
                .with_placeholder(keys::SAVES_NAME_PLACEHOLDER)
                .with_max_characters(super::container::MAX_NAME_CHARS)
                .full_width()
                .spawn_child(r);
            r.commands()
                .entity(field)
                .insert((NameField, UiId::new("saves.name")));
            let save = ButtonBuilder::filled(keys::SAVES_SAVE)
                .with_key(keys::SAVES_SAVE)
                .spawn_child(r);
            r.commands()
                .entity(save)
                .insert((SavesButton::Save, UiId::new("saves.save")));
            let rename = ButtonBuilder::filled(keys::SAVES_RENAME_APPLY)
                .with_key(keys::SAVES_RENAME_APPLY)
                .spawn_child(r);
            r.commands().entity(rename).insert((
                SavesButton::ApplyRename,
                UiId::new("saves.rename-apply"),
                Node {
                    display: Display::None,
                    ..default()
                },
            ));
            r.spawn((
                Text::new(keys::SAVES_NAME_EMPTY),
                L10nKey(keys::SAVES_NAME_EMPTY),
                TextColor::default(),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::Error),
                Node {
                    display: Display::None,
                    ..default()
                },
                NameError,
                UiId::new("saves.name-error"),
            ));
        });
        c.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            SavesList,
            UiId::new("saves.list"),
        ));
        let close = ButtonBuilder::tonal(keys::HELP_CLOSE)
            .with_key(keys::HELP_CLOSE)
            .spawn_child(c);
        c.commands().entity(close).insert((
            SavesButton::Close,
            UiId::new("saves.close"),
            Node {
                align_self: AlignSelf::FlexEnd,
                ..default()
            },
        ));
    });
}

/// Esc closes the overlay (the topmost modal answers first, so a dialog above it closes alone).
pub fn close_on_cancel(
    mut commands: Commands,
    mut cancels: MessageReader<ModalEvent>,
    overlays: Query<Entity, With<SavesOverlay>>,
) {
    for ModalEvent::CancelRequested(modal) in cancels.read() {
        if overlays.contains(*modal) {
            commands.entity(*modal).despawn();
        }
    }
}

pub(super) fn close_overlay(world: &mut World) {
    let roots: Vec<Entity> = world
        .query_filtered::<Entity, With<SavesOverlay>>()
        .iter(world)
        .collect();
    for e in roots {
        world.entity_mut(e).despawn();
    }
}

/// Clicks on the overlay's own buttons.
pub fn handle_buttons(
    mut activated: MessageReader<Activated>,
    buttons: Query<&SavesButton>,
    mut commands: Commands,
) {
    for Activated(entity) in activated.read() {
        let Ok(button) = buttons.get(*entity).cloned() else {
            continue;
        };
        commands.queue(move |world: &mut World| match button {
            SavesButton::Close => close_overlay(world),
            SavesButton::Save => submit_name(world),
            SavesButton::ApplyRename => apply_rename(world),
            SavesButton::Row(id, action) => dialogs::row_action(world, &id, action),
        });
    }
}

fn field_text(world: &mut World) -> String {
    world
        .query_filtered::<&M3TextField, With<NameField>>()
        .iter(world)
        .next()
        .map(|f| f.text.clone())
        .unwrap_or_default()
}

pub(super) fn set_field_text(world: &mut World, text: &str) {
    for mut f in world
        .query_filtered::<&mut M3TextField, With<NameField>>()
        .iter_mut(world)
    {
        f.text = text.to_owned();
    }
}

/// *Save* with the name in the field: asks before replacing a save of the same name.
fn submit_name(world: &mut World) {
    let name = field_text(world);
    let request = SaveRequest {
        name: name.clone(),
        id: None,
        overwrite: false,
    };
    match save_session(world, &request) {
        Ok(saved) => {
            set_field_text(world, "");
            dialogs::saved_toast(world, &saved.name);
            let mut ui = world.resource_mut::<SavesUi>();
            ui.dirty = true;
            ui.name_error = None;
        }
        Err(SaveError::Exists(id)) => dialogs::ask_overwrite(world, Some(id), name),
        Err(SaveError::InvalidName(_)) => {
            world.resource_mut::<SavesUi>().name_error = Some(keys::SAVES_NAME_EMPTY);
        }
        Err(e) => dialogs::save_failed(world, &e),
    }
}

fn apply_rename(world: &mut World) {
    let Some(id) = world.resource_mut::<SavesUi>().renaming.take() else {
        return;
    };
    let name = field_text(world);
    match super::ops::store(world).and_then(|s| s.rename(&id, &name)) {
        Ok(()) => {
            set_field_text(world, "");
            let mut ui = world.resource_mut::<SavesUi>();
            ui.dirty = true;
            ui.name_error = None;
        }
        Err(SaveError::InvalidName(_)) => {
            let mut ui = world.resource_mut::<SavesUi>();
            ui.renaming = Some(id);
            ui.name_error = Some(keys::SAVES_NAME_EMPTY);
        }
        Err(e) => dialogs::save_failed(world, &e),
    }
}

/// Keeps the name row, its buttons and its error in step with the overlay's state.
pub fn sync_name_row(
    ui: Res<SavesUi>,
    mut rows: Query<&mut Node, (With<NameRow>, Without<NameError>)>,
    mut errors: Query<&mut Node, (With<NameError>, Without<NameRow>)>,
    buttons: Query<(&SavesButton, Entity)>,
    mut nodes: Query<&mut Node, (Without<NameRow>, Without<NameError>)>,
) {
    if !ui.is_changed() {
        return;
    }
    let visible = ui.mode == SavesMode::Save || ui.renaming.is_some();
    for mut node in &mut rows {
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut node in &mut errors {
        node.display = if ui.name_error.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (button, entity) in &buttons {
        let show = match button {
            SavesButton::Save => ui.renaming.is_none(),
            SavesButton::ApplyRename => ui.renaming.is_some(),
            _ => continue,
        };
        if let Ok(mut node) = nodes.get_mut(entity) {
            node.display = if show { Display::Flex } else { Display::None };
        }
    }
}

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
