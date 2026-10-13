//! The rail footer: theme toggle (F2) and *Quit OPTS* with its confirmation.

use bevy::app::AppExit;
use bevy::prelude::*;
use opts_gui::interaction::activation::Activated;
use opts_gui::interaction::tooltip::Tooltip;
use opts_gui::l10n::{L10nArgs, L10nKey, L10nTooltip};
use opts_gui::prelude::*;
use opts_gui::theme::ThemeMode;

use super::dialog::{
    spawn_confirm_dialog, DialogButtonSpec, DialogButtonStyle, DialogChoice, DialogSpec, CANCEL,
};
use super::frame::{RailAction, RailIcon};
use super::{icons, keys};
use crate::settings::GameSettings;

/// Tag of the quit confirmation.
pub const QUIT_DIALOG: &str = "shell.quit";
/// The choice that quits.
pub const QUIT: &str = "quit";

/// F2 or a click on the footer toggle switches between Dark and Light.
///
/// The toggle is the quick switch; System and Auto (clock) are chosen in the settings. Choosing it
/// leaves the high-contrast switch off.
pub fn theme_toggle_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut activated: MessageReader<Activated>,
    actions: Query<&RailAction>,
    theme: Res<MaterialTheme>,
    mut settings: ResMut<GameSettings>,
) {
    let mut toggle = keyboard.just_pressed(KeyCode::F2);
    for Activated(entity) in activated.read() {
        toggle |= actions.get(*entity) == Ok(&RailAction::ThemeToggle);
    }
    if toggle {
        let mode = if theme.kind().is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        tracing::info!(?mode, "Toggling application theme");
        settings.theme_mode = mode;
        settings.high_contrast = false;
        settings.save_in_background();
    }
}

/// The footer toggle offers the mode a click switches **to**.
#[allow(clippy::too_many_arguments)]
pub fn sync_theme_toggle(
    theme: Res<MaterialTheme>,
    toggles: Query<(Entity, &RailAction)>,
    children: Query<&Children>,
    mut icons_q: Query<&mut Text, With<RailIcon>>,
    mut labels: Query<&mut L10nKey, Without<RailIcon>>,
    mut commands: Commands,
    added: Query<(), Added<RailAction>>,
    loc: Res<opts_gui::l10n::Localization>,
) {
    if !theme.is_changed() && added.is_empty() {
        return;
    }
    let (key, glyph) = if theme.kind().is_dark() {
        (keys::THEME_TO_LIGHT, icons::LIGHT_MODE)
    } else {
        (keys::THEME_TO_DARK, icons::DARK_MODE)
    };
    for (entity, action) in &toggles {
        if *action != RailAction::ThemeToggle {
            continue;
        }
        commands
            .entity(entity)
            .insert((L10nTooltip(key), Tooltip::new(loc.t(key))));
        for child in children.iter_descendants(entity) {
            if let Ok(mut text) = icons_q.get_mut(child) {
                if text.0 != glyph {
                    text.0 = glyph.to_owned();
                }
            }
            if let Ok(mut label) = labels.get_mut(child) {
                if label.0 != opts_gui::l10n::keys::NOT_AVAILABLE && label.0 != key {
                    label.0 = key;
                }
            }
        }
    }
}

/// The dialog for quitting the application from a shell screen.
pub fn quit_dialog_spec() -> DialogSpec {
    DialogSpec {
        tag: QUIT_DIALOG,
        title_key: keys::QUIT_TITLE,
        body_key: keys::QUIT_BODY,
        body_args: L10nArgs::default(),
        buttons: vec![
            DialogButtonSpec {
                choice: CANCEL,
                label_key: keys::CANCEL,
                style: DialogButtonStyle::Quiet,
            },
            DialogButtonSpec {
                choice: QUIT,
                label_key: keys::QUIT_CONFIRM,
                style: DialogButtonStyle::Destructive,
            },
        ],
        keep_open: Vec::new(),
        error: None,
    }
}

/// *Quit OPTS* in the rail asks first.
pub fn quit_click_system(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    actions: Query<&RailAction>,
    open: Query<(), With<super::dialog::ConfirmDialog>>,
) {
    for Activated(entity) in activated.read() {
        if actions.get(*entity) == Ok(&RailAction::Quit) && open.is_empty() {
            spawn_confirm_dialog(&mut commands, quit_dialog_spec());
        }
    }
}

/// The confirmed quit leaves the application.
pub fn quit_choice_system(
    mut choices: MessageReader<DialogChoice>,
    mut exit: MessageWriter<AppExit>,
) {
    for choice in choices.read() {
        if choice.tag == QUIT_DIALOG && choice.choice == QUIT {
            tracing::info!("Quit confirmed; requesting a clean exit");
            exit.write(AppExit::Success);
        }
    }
}
