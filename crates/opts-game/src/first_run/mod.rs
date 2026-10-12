//! First run: confirm or choose the OMSI folder, read the content, welcome
//! (`docs/src/roadmap/phase-9/screens.md` section 3).
//!
//! * A folder found by searching is **offered for confirmation**, never adopted silently
//!   ([`decision::decide`]); an explicit choice (command line, `$OMSI_PATH`, the folder confirmed
//!   earlier) is used as it is.
//! * [`choose_omsi_folder`] opens the native folder dialog; `Settings -> Content`, the Home banner
//!   and this flow all use it. A valid choice is saved and the catalogue starts at once; an
//!   invalid one explains what a valid folder contains.
//! * When the content has been read for the first time and there is no drive history, one
//!   welcome dialog offers *Pick a map and a vehicle* or *Quick drive*, once.

pub mod decision;
mod view;

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Mutex;

use bevy::app::AppExit;
use bevy::prelude::*;
use opts_gui::l10n::L10nArgs;

pub use decision::{check_choice, decide, ChooseError, StartupDecision, StartupFacts};

use crate::app_state::AppState;
use crate::catalogue_service::{start_catalogue_service, CatalogueService, DiscoveryStatus};
use crate::drive_history::DriveRecord;
use crate::home::{HomeDrives, StartDrive};
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::omsi_root::set_saved_root;
use crate::settings::GameSettings;
use crate::shell::banner::{dismiss_banner, show_banner, BannerSeverity, BannerSpec};
use crate::shell::dialog::{DialogChoice, CANCEL};
use crate::shell::{keys, nav::Navigate};

/// Tag of the first-run dialogs.
pub const DIALOG_TAG: &str = "first-run";
/// Banner id of a rejected folder.
const BANNER_FOLDER: &str = "omsi.folder-error";
/// Banner id of settings that could not be saved.
const BANNER_SAVE: &str = "omsi.save-failed";

/// The step the first-run flow shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// An installation was found in a default location: confirm it.
    Found(PathBuf),
    /// Nothing was found: choose a folder.
    Choose,
    /// The content was read: welcome.
    Welcome,
}

/// State of the flow.
#[derive(Resource, Debug)]
pub struct FirstRun {
    /// The open step, if any.
    pub step: Option<Step>,
    /// Why the last chosen folder was refused (shown inline in the dialog).
    pub error: Option<ChooseError>,
    /// The welcome was shown (or is not needed): it never comes back.
    pub welcome_done: bool,
    /// Whether `welcome_done` is read from and written to the user's data directory.
    pub persist: bool,
}

impl Default for FirstRun {
    fn default() -> Self {
        Self {
            step: None,
            error: None,
            welcome_done: false,
            persist: true,
        }
    }
}

fn state_file() -> Option<PathBuf> {
    GameSettings::database_path().map(|p| p.with_file_name("first_run.json"))
}

impl FirstRun {
    /// The remembered state: whether the welcome was shown.
    pub fn load() -> Self {
        let done = state_file()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("welcome_done").and_then(serde_json::Value::as_bool))
            .unwrap_or(false);
        Self {
            welcome_done: done,
            ..Self::default()
        }
    }

    fn finish_welcome(&mut self) {
        self.step = None;
        self.welcome_done = true;
        if !self.persist {
            return;
        }
        if let Some(path) = state_file() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(path, r#"{"welcome_done":true}"#) {
                tracing::warn!("could not remember that the welcome was shown: {e}");
            }
        }
    }
}

/// The answer of the folder dialog (also sent when the choice came from Settings or Home): the
/// installation root, already saved and in use, or the reason it was refused.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct OmsiFolderChosen(pub Result<PathBuf, ChooseError>);

/// The open native dialog, if any.
#[derive(Resource, Default)]
pub struct FolderPicker(Mutex<Option<Receiver<Option<PathBuf>>>>);

impl FolderPicker {
    /// Whether a dialog is open (or its answer is not taken yet).
    pub fn is_open(&self) -> bool {
        self.0.lock().is_ok_and(|slot| slot.is_some())
    }
}

/// What shows the folder dialog (the native one; tests substitute a fake that answers at once).
#[derive(Resource, Clone)]
pub struct FolderOpener(pub std::sync::Arc<dyn Fn() -> Option<PathBuf> + Send + Sync>);

impl Default for FolderOpener {
    fn default() -> Self {
        Self(std::sync::Arc::new(|| {
            rfd::FileDialog::new()
                .set_title(select_title())
                .pick_folder()
        }))
    }
}

/// Opens the native folder dialog on its own thread. Anyone may call it (the Settings page for
/// *Content -> OMSI folder*, the Home banner); the result arrives as [`OmsiFolderChosen`].
pub fn choose_omsi_folder(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        let opener = world
            .get_resource::<FolderOpener>()
            .cloned()
            .unwrap_or_default();
        let picker = world.get_resource_or_insert_with(FolderPicker::default);
        let Ok(mut slot) = picker.0.lock() else {
            return;
        };
        if slot.is_some() {
            return; // a dialog is already open
        }
        let (tx, rx) = channel();
        *slot = Some(rx);
        std::thread::spawn(move || {
            let _ = tx.send((opener.0)());
        });
    });
}

/// Title of the native dialog (the OS dialog is outside the message tables).
fn select_title() -> &'static str {
    "OMSI 2" // l10n-ok: the product name; the dialog's own chrome is the OS's
}

/// Takes the dialog's answer, saves a valid folder and starts the catalogue.
#[allow(clippy::too_many_arguments)]
pub fn poll_folder_pick(
    mut commands: Commands,
    picker: Option<Res<FolderPicker>>,
    mut settings: ResMut<GameSettings>,
    mut first_run: ResMut<FirstRun>,
    mut chosen: MessageWriter<OmsiFolderChosen>,
) {
    let Some(picker) = picker else { return };
    let answer = {
        let Ok(mut slot) = picker.0.lock() else {
            return;
        };
        let Some(rx) = slot.as_ref() else { return };
        match rx.try_recv() {
            Ok(answer) => {
                *slot = None;
                answer
            }
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                *slot = None;
                None
            }
        }
    };
    match check_choice(answer) {
        Ok(root) => {
            adopt_root(&mut commands, &mut settings, &root, first_run.persist);
            if matches!(first_run.step, Some(Step::Found(_) | Step::Choose)) {
                first_run.step = None;
            }
            first_run.error = None;
            dismiss_banner(&mut commands, BANNER_FOLDER);
            chosen.write(OmsiFolderChosen(Ok(root)));
        }
        Err(ChooseError::Cancelled) => {
            chosen.write(OmsiFolderChosen(Err(ChooseError::Cancelled)));
        }
        Err(error @ ChooseError::NotInstallation(_)) => {
            let ChooseError::NotInstallation(path) = &error else {
                return;
            };
            // Durable: the dialog shows it inline; elsewhere a banner keeps it until resolved.
            if first_run.step.is_some() {
                first_run.step = Some(Step::Choose);
                first_run.error = Some(error.clone());
            } else {
                show_banner(
                    &mut commands,
                    BannerSpec {
                        id: BANNER_FOLDER,
                        severity: BannerSeverity::Error,
                        title_key: keys::FIRSTRUN_ERROR_TITLE,
                        body_key: keys::FIRSTRUN_WRONG_FOLDER,
                        args: L10nArgs::one("path", path.display().to_string()),
                        action: None,
                        owner: None,
                    },
                );
            }
            chosen.write(OmsiFolderChosen(Err(error)));
        }
    }
}

/// Saves `root` as the confirmed installation and (re)starts the catalogue with it.
pub fn adopt_root(
    commands: &mut Commands,
    settings: &mut GameSettings,
    root: &std::path::Path,
    persist: bool,
) {
    tracing::info!(path = %root.display(), "OMSI installation confirmed");
    settings.omsi_path = Some(root.to_path_buf());
    set_saved_root(Some(root.to_path_buf()));
    if !persist {
        return;
    }
    if let Err(e) = settings.save() {
        tracing::warn!("could not save the OMSI path: {e}");
        show_banner(
            commands,
            BannerSpec {
                id: BANNER_SAVE,
                severity: BannerSeverity::Warning,
                title_key: keys::FIRSTRUN_SAVE_FAILED_TITLE,
                body_key: keys::FIRSTRUN_SAVE_FAILED_BODY,
                args: L10nArgs::one("error", e.to_string()),
                action: None,
                owner: None,
            },
        );
    } else {
        dismiss_banner(commands, BANNER_SAVE);
    }
    commands.run_system_cached(start_catalogue_service);
}

/// Reacts to the buttons of the first-run dialogs.
#[allow(clippy::too_many_arguments)]
pub fn first_run_choices(
    mut commands: Commands,
    mut choices: MessageReader<DialogChoice>,
    mut first_run: ResMut<FirstRun>,
    mut settings: ResMut<GameSettings>,
    mut loc: ResMut<opts_gui::l10n::Localization>,
    setup: Res<NewGameSetup>,
    mut navigate: MessageWriter<Navigate>,
    mut start: MessageWriter<StartDrive>,
    mut exit: MessageWriter<AppExit>,
) {
    for choice in choices.read().filter(|c| c.tag == DIALOG_TAG) {
        match (choice.choice, first_run.step.clone()) {
            (view::USE, Some(Step::Found(root))) => {
                adopt_root(&mut commands, &mut settings, &root, first_run.persist);
                first_run.step = None;
            }
            (view::LANGUAGE, _) => {
                // Offered on every step (M-0065): the other language, kept like the setting.
                let next = if loc.locale().starts_with("de") {
                    "en"
                } else {
                    "de"
                };
                loc.set_locale(next);
                settings.set_raw("interface.language", next);
                if first_run.persist {
                    if let Err(e) = settings.save() {
                        tracing::warn!("could not save the language: {e}");
                    }
                }
            }
            (view::OTHER | view::CHOOSE, _) => choose_omsi_folder(&mut commands),
            (view::QUIT, _) => {
                exit.write(AppExit::Success);
            }
            (view::PICK, Some(Step::Welcome)) => {
                first_run.finish_welcome();
                navigate.write(Navigate::push("drive"));
            }
            (view::QUICK, Some(Step::Welcome)) => {
                first_run.finish_welcome();
                start.write(StartDrive(DriveRecord::from_setup(&setup, 0)));
            }
            (CANCEL, Some(Step::Welcome)) => first_run.finish_welcome(),
            (CANCEL, Some(_)) => first_run.step = None,
            _ => {}
        }
    }
}

/// Shows the welcome once the content is read and nothing else is open.
pub fn welcome_system(
    state: Res<State<AppState>>,
    service: Option<Res<CatalogueService>>,
    lists: Res<GameCatalogue>,
    drives: Res<HomeDrives>,
    mut first_run: ResMut<FirstRun>,
) {
    if first_run.step.is_some() || first_run.welcome_done || *state.get() != AppState::MainMenu {
        return;
    }
    let ready = service.is_some_and(|s| !matches!(s.status(), DiscoveryStatus::Running(_)))
        && !lists.maps.is_empty()
        && !lists.vehicles.is_empty();
    if decision::welcome_due(
        first_run.welcome_done,
        !drives.history.recents.is_empty(),
        ready,
    ) {
        first_run.step = Some(Step::Welcome);
    }
}

/// Starting a drive (or any other way out of Home) ends the welcome: it never comes back.
pub fn close_welcome_on_leaving_home(mut first_run: ResMut<FirstRun>) {
    if first_run.step == Some(Step::Welcome) {
        first_run.finish_welcome();
    }
}

/// First-run plugin.
pub struct FirstRunPlugin;

impl Plugin for FirstRunPlugin {
    fn build(&self, app: &mut App) {
        if !app.world().contains_resource::<FirstRun>() {
            app.insert_resource(FirstRun::load());
        }
        app.init_resource::<FolderPicker>()
            .init_resource::<HomeDrives>()
            .init_resource::<GameCatalogue>()
            .init_resource::<NewGameSetup>()
            .add_message::<OmsiFolderChosen>()
            .add_message::<StartDrive>()
            .add_systems(OnExit(AppState::MainMenu), close_welcome_on_leaving_home)
            .add_systems(
                Update,
                (
                    poll_folder_pick,
                    welcome_system,
                    first_run_choices,
                    view::sync_first_run_dialog,
                )
                    .chain(),
            );
    }
}

#[cfg(test)]
mod tests;
