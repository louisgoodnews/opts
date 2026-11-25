//! The parsed command line as a Bevy resource (split out of `opts-game`'s `app` module, ISS-064
//! stage 3). `opts-game` re-exports it as `opts_game::AppArgs`, so consumer paths do not change.

use bevy::prelude::*;
use opts_cli::CliArgs;

/// Resource storing the parsed CLI arguments.
#[derive(Resource, Debug, Clone)]
pub struct AppArgs(pub CliArgs);
