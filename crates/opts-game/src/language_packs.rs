//! Language packs in the running game: scan at start-up and on demand, the control methods
//! `l10n.languages`, `l10n.set`, `l10n.reload`, `l10n.report`, and the *Reload languages* action
//! of the settings page.
//!
//! The format, the checks and the folder layout are in `opts_l10n` and documented in
//! `docs/src/architecture/localisation-packs.md`; the merge with the built-in tables is
//! `opts_gui::l10n::Localization::load_packs`. A broken pack never stops the game: its problems
//! are logged and listed by `l10n.report`, and the English text shows where a message is
//! unusable.

mod methods;
mod scan;
mod status;

pub use methods::register;
pub use scan::{load_startup, scan, scan_dir, ScanSummary};
pub use status::info_json;

use std::path::PathBuf;

use bevy::prelude::*;
use opts_gui::components::toast::ToastBuilder;
use opts_gui::l10n::Localization;

use crate::settings_screen::SettingActionRequested;

/// The settings key of the language.
pub const LANGUAGE_KEY: &str = "interface.language";
/// The settings action that scans the language folder again.
pub const RELOAD_KEY: &str = "interface.language_reload";

/// The folder language packs are read from (`<user data dir>/lang`).
#[must_use]
pub fn packs_dir() -> Option<PathBuf> {
    opts_l10n::pack::default_root()
}

/// The *Reload languages* button of the settings page.
pub fn reload_action(mut requests: MessageReader<SettingActionRequested>, mut commands: Commands) {
    if requests.read().any(|r| r.key == RELOAD_KEY) {
        commands.queue(|world: &mut World| {
            let summary = {
                let mut loc = world.resource_mut::<Localization>();
                scan(&mut loc)
            };
            let loc = world.resource::<Localization>();
            let mut text = loc.t_args(
                "settings-toast-languages-reloaded",
                &[("count".into(), summary.usable.into())],
            );
            if summary.broken + summary.errors > 0 {
                text.push(' ');
                text.push_str(&loc.t_args(
                    "settings-toast-languages-problems",
                    &[("count".into(), (summary.broken + summary.errors).into())],
                ));
            }
            let mut commands = world.commands();
            ToastBuilder::new(text)
                .with_duration(5.0)
                .spawn(&mut commands);
        });
    }
}
