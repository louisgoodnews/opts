//! The loading screen on self-authored miniature installations: stages, the manifest of the real
//! session, seeded failures that must stay visible, and the three outcomes.

mod common;
#[path = "common/loading.rs"]
mod loading;
#[path = "common/simple_map.rs"]
mod simple_map;
#[path = "loading_tests/helpers.rs"]
mod helpers;

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    in_game::{
        load_report::{LoadOutcome, LoadReport},
        CancelLoadingBtn, InGameEntity, InGameHudRoot, LoadStage, SimulationLoadState,
    },
    CliArgs,
};
use opts_gui::interaction::Activated;
use opts_gui::ui_tree::find_entity;

use helpers::{has, loading_app, outcome};
use loading::{args_for, run, screen_count, texts};
use common::{fresh_root, write_map, MiniMap};

#[path = "loading_tests/accepted.rs"]
mod accepted;
#[path = "loading_tests/missing.rs"]
mod missing;
#[path = "loading_tests/screen.rs"]
mod screen;

fn broken_tile_root(tag: &str) -> std::path::PathBuf {
    let root = fresh_root(tag);
    write_map(
        &root,
        &MiniMap {
            name: "Mini",
            listed: vec![(0, 0), (1, 0)],
            written: vec![(0, 0)],
            dangling_objects: 0,
        },
    );
    root
}

#[path = "loading_tests/camera.rs"]
mod camera;
#[path = "loading_tests/minor.rs"]
mod minor;
#[path = "loading_tests/outcomes.rs"]
mod outcomes;
