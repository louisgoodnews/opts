//! Startup check for the OMSI 2 installation (`docs/src/architecture/application-startup.md`).
//!
//! The root comes from `--omsi-path`/`--map-path`, `$OMSI_PATH`, the folder the user confirmed
//! earlier (settings) and, as a *candidate*, the default Steam locations. An explicit choice or a
//! confirmed folder is used as it is. A candidate is **offered for confirmation** by the first-run
//! dialogs ([`crate::first_run`]); when nothing is found the user chooses with the native folder
//! dialog. Without a window (`--headless`, `--autostart`) nothing is asked: the search result is
//! used and remembered, and the catalogue stays empty when there is none.

use std::path::PathBuf;

use bevy::prelude::*;

use crate::app::AppArgs;
use crate::catalogue_service::start_catalogue_service;
use crate::first_run::{decide, FirstRun, FirstRunPlugin, StartupDecision, StartupFacts, Step};
use crate::omsi_root::{candidate_root, require_confirmation, set_saved_root};
use crate::settings::GameSettings;

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

fn has_maps(path: PathBuf) -> Option<PathBuf> {
    path.join("maps").is_dir().then_some(path)
}

/// What the start-up check knows about the installation.
pub fn startup_facts(settings: &GameSettings, args: Option<&AppArgs>) -> StartupFacts {
    let interactive = args.is_some_and(|a| !a.0.headless && !a.0.autostart);
    StartupFacts {
        explicit: explicit_root(args).and_then(has_maps),
        env: std::env::var_os("OMSI_PATH")
            .map(PathBuf::from)
            .and_then(has_maps),
        saved: settings
            .omsi_path
            .clone()
            .filter(|p| crate::omsi_root::is_omsi_root(p)),
        candidate: candidate_root(),
        interactive,
    }
}

/// Startup system: decides what to do with the installation found (see [`decide`]).
pub fn resolve_omsi_on_startup(
    mut settings: ResMut<GameSettings>,
    args: Option<Res<AppArgs>>,
    mut first_run: ResMut<FirstRun>,
) {
    let facts = startup_facts(&settings, args.as_deref());
    // An interactive start must not use a found folder before it is confirmed.
    require_confirmation(facts.interactive);
    set_saved_root(settings.omsi_path.clone());
    match decide(&facts) {
        StartupDecision::Adopt { root, remember } => {
            if remember && settings.omsi_path.as_deref() != Some(root.as_path()) {
                info!(path = %root.display(), "OMSI installation found; remembering it");
                settings.omsi_path = Some(root);
                if let Err(e) = settings.save() {
                    warn!("could not save the OMSI path: {e}");
                }
            }
        }
        StartupDecision::Confirm(root) => {
            info!(path = %root.display(), "OMSI installation found; asking for confirmation");
            first_run.step = Some(Step::Found(root));
        }
        StartupDecision::Choose => first_run.step = Some(Step::Choose),
        StartupDecision::Missing => warn!(
            "OMSI installation not found (pass --omsi-path or set $OMSI_PATH); catalogue is empty"
        ),
    }
}

/// Registers the startup check (before the catalogue service) and the first-run flow.
pub struct OmsiSetupPlugin;

impl Plugin for OmsiSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FirstRunPlugin).add_systems(
            Startup,
            resolve_omsi_on_startup.before(start_catalogue_service),
        );
    }
}
