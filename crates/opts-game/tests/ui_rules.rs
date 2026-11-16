//! The CI rules of `expandability.md` section 7 on the screens that exist today.
//!
//! Home and the rail are held to every rule. The other screens (new game, settings, HUD, loading) still
//! have hard-coded 11 px texts and small targets; they are recorded in
//! `docs/dev/progress/ui-widgets.md` and join this file when the shell milestone rebuilds them.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState, help::HelpPlugin, home::HomePlugin, settings::GameSettings,
    shell::ShellPlugin,
};
use opts_gui::prelude::*;
use opts_gui::testing::rules;

#[path = "ui_rules/shell.rs"]
mod shell;
#[path = "ui_rules/overlays.rs"]
mod overlays;
