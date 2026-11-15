//! The loading screen on self-authored miniature installations: stages, the manifest of the real
//! session, seeded failures that must stay visible, and the three outcomes.

mod common;

use bevy::prelude::*;
use opts_game::{
    in_game::{
        load_report::{LoadOutcome, LoadReport},
        LoadStage, SimulationLoadState,
    },
    CliArgs,
};
use opts_gui::interaction::Activated;
use opts_gui::ui_tree::find_entity;

use common::loading::{args_for, has, loading_app, outcome, run, screen_count, texts};
use common::{fresh_root, write_map, MiniMap};

#[path = "loading_tests/stages.rs"]
mod stages;
#[path = "loading_tests/tiles.rs"]
mod tiles;
#[path = "loading_tests/problems.rs"]
mod problems;
#[path = "loading_tests/flow.rs"]
mod flow;
#[path = "loading_tests/outcomes.rs"]
mod outcomes;
