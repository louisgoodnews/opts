//! Startup check for the OMSI 2 installation (`docs/src/architecture/application-startup.md`).
//!
//! The root is looked up through `--omsi-path`/`--map-path`, `$OMSI_PATH`, the path saved in the
//! settings and the default Steam locations. A root found without an explicit override is saved
//! to the settings, so later runs use the same installation. When nothing is found, a modal asks
//! the user to pick the folder in a file dialog; a valid choice is saved and the catalogue
//! starts. Without a window (`--headless`, `--autostart`) nothing is asked and the catalogue
//! stays empty, as before.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Mutex;

use bevy::prelude::*;
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::prelude::*;

use crate::app::AppArgs;
use crate::catalogue_service::start_catalogue_service;
use crate::omsi_root::{locate_omsi_root, root_from_selection, set_saved_root};
use crate::settings::GameSettings;

/// The open dialog (present while the user has not chosen an installation yet).
#[derive(Resource)]
pub struct OmsiSetupPrompt {
    modal: Entity,
    pick: Mutex<Option<Receiver<Option<PathBuf>>>>,
}

#[derive(Component)]
struct BrowseButton;

#[derive(Component)]
struct QuitButton;

#[derive(Component)]
struct SetupMessage;

/// The explicit root of this run: `--omsi-path` or the root above `--map-path`.
fn explicit_root(args: Option<&AppArgs>) -> Option<PathBuf> {
    let args = args?;
    args.0.omsi_path.clone().or_else(|| {
        args.0
            .map_path
            .as_deref()
            .and_then(opts_bevy::OmsiInstall::root_from_map_dir)
    })
}

/// Startup system: finds (and remembers) the installation or opens the picker dialog.
pub fn resolve_omsi_on_startup(
    mut commands: Commands,
    mut settings: ResMut<GameSettings>,
    args: Option<Res<AppArgs>>,
) {
    set_saved_root(settings.omsi_path.clone());
    let explicit = explicit_root(args.as_deref());
    match locate_omsi_root(explicit.as_deref()) {
        Some(root) => {
            let overridden = explicit.is_some() || std::env::var_os("OMSI_PATH").is_some();
            if !overridden && settings.omsi_path.as_deref() != Some(root.as_path()) {
                info!(path = %root.display(), "OMSI installation found; remembering it");
                settings.omsi_path = Some(root);
                if let Err(e) = settings.save() {
                    warn!("could not save the OMSI path: {e}");
                }
            }
        }
        None => {
            let interactive = args
                .as_ref()
                .is_some_and(|a| !a.0.headless && !a.0.autostart);
            if interactive {
                spawn_prompt(&mut commands);
            } else {
                warn!(
                    "OMSI installation not found (pass --omsi-path or set $OMSI_PATH); catalogue is empty"
                );
            }
        }
    }
}

pub(crate) fn spawn_prompt(commands: &mut Commands) {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(50.0, 40.0)
        .spawn(commands);
    commands.entity(content).with_children(|p| {
        p.spawn((
            Text::new("OMSI 2 installation not found"),
            TextFont {
                font_size: FontSize::Px(22.0),
                ..default()
            },
            TextColor(Color::WHITE),
            ForegroundRole(ColorRole::OnSurface),
        ));
        p.spawn((
            Text::new(
                "OPTS reads its maps, vehicles and objects from your own OMSI 2 installation. \
                 Choose the folder that contains \"maps\" and \"Vehicles\"; it is remembered \
                 for the next start.",
            ),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::WHITE),
            ForegroundRole(ColorRole::OnSurfaceVariant),
        ));
        p.spawn((
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(14.0),
                ..default()
            },
            TextColor(Color::WHITE),
            ForegroundRole(ColorRole::Error),
            SetupMessage,
        ));
        p.spawn(Node {
            column_gap: Val::Px(12.0),
            justify_content: JustifyContent::FlexEnd,
            ..default()
        })
        .with_children(|row| {
            let browse = ButtonBuilder::filled("Browse…").spawn_child(row);
            let quit = ButtonBuilder::outlined("Quit").spawn_child(row);
            row.commands().queue(move |world: &mut World| {
                world.entity_mut(browse).insert(BrowseButton);
                world.entity_mut(quit).insert(QuitButton);
            });
        });
    });
    commands.insert_resource(OmsiSetupPrompt {
        modal: root,
        pick: Mutex::new(None),
    });
}

/// Browse opens the folder dialog on its own thread; Quit leaves.
fn handle_setup_buttons(
    mut clicks: MessageReader<ButtonClicked>,
    browse: Query<(), With<BrowseButton>>,
    quit: Query<(), With<QuitButton>>,
    prompt: Res<OmsiSetupPrompt>,
    mut exit: MessageWriter<AppExit>,
) {
    for click in clicks.read() {
        if quit.get(click.0).is_ok() {
            exit.write(AppExit::Success);
        } else if browse.get(click.0).is_ok() {
            let Ok(mut slot) = prompt.pick.lock() else {
                continue;
            };
            if slot.is_some() {
                continue; // a dialog is already open
            }
            let (tx, rx) = channel();
            *slot = Some(rx);
            std::thread::spawn(move || {
                let picked = rfd::FileDialog::new()
                    .set_title("Select your OMSI 2 folder")
                    .pick_folder();
                let _ = tx.send(picked);
            });
        }
    }
}

/// Validates the dialog's answer: the installation root, or the message to show.
fn check_choice(answer: Option<PathBuf>) -> Result<PathBuf, String> {
    let picked = answer.ok_or_else(|| "No folder was selected.".to_owned())?;
    root_from_selection(&picked).ok_or_else(|| {
        format!(
            "\"{}\" is not an OMSI 2 installation: it needs the folders \"maps\" and \"Vehicles\".",
            picked.display()
        )
    })
}

/// Handles the dialog's answer: saves a valid installation and starts the catalogue.
fn poll_setup_pick(
    mut commands: Commands,
    prompt: Res<OmsiSetupPrompt>,
    mut settings: ResMut<GameSettings>,
    mut message: Query<&mut Text, With<SetupMessage>>,
) {
    let answer = {
        let Ok(mut slot) = prompt.pick.lock() else {
            return;
        };
        let Some(rx) = slot.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(answer) => {
                *slot = None;
                answer
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                *slot = None;
                None
            }
        }
    };
    let say = |text: &str, message: &mut Query<&mut Text, With<SetupMessage>>| {
        for mut t in message {
            t.0 = text.to_owned();
        }
    };
    let root = match check_choice(answer) {
        Ok(root) => root,
        Err(text) => {
            say(&text, &mut message);
            return;
        }
    };
    info!(path = %root.display(), "OMSI installation chosen by the user");
    settings.omsi_path = Some(root.clone());
    if let Err(e) = settings.save() {
        warn!("could not save the OMSI path: {e}");
    }
    set_saved_root(Some(root));
    commands.entity(prompt.modal).despawn();
    commands.remove_resource::<OmsiSetupPrompt>();
    commands.run_system_cached(start_catalogue_service);
}

/// Registers the startup check (before the catalogue service) and the dialog systems.
pub struct OmsiSetupPlugin;

impl Plugin for OmsiSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            resolve_omsi_on_startup.before(start_catalogue_service),
        )
        .add_systems(
            Update,
            (handle_setup_buttons, poll_setup_pick).run_if(resource_exists::<OmsiSetupPrompt>),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelled_and_invalid_choices_explain_themselves() {
        assert_eq!(
            check_choice(None),
            Err("No folder was selected.".to_owned())
        );
        let err = check_choice(Some(std::env::temp_dir())).expect_err("not an install");
        assert!(err.contains("maps") && err.contains("Vehicles"), "{err}");
    }

    #[test]
    fn a_valid_choice_resolves_to_the_root() {
        let root = std::env::temp_dir().join(format!("opts-setup-{}", std::process::id()));
        std::fs::create_dir_all(root.join("maps/M")).expect("mkdir");
        std::fs::create_dir_all(root.join("Vehicles")).expect("mkdir");
        assert_eq!(check_choice(Some(root.join("maps/M"))), Ok(root.clone()));
        let _ = std::fs::remove_dir_all(root);
    }
}
