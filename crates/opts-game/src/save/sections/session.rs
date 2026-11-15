//! The `session` and `weather` save sections: the map, the start point, play time and the weather.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::super::registry::{CheckEnv, Issue, IssueKind};
use super::super::{fingerprint, PlayClock};
use super::{parse, to_value};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::in_game::types::InGameSessionInfo;
use crate::new_game_screen::types::NewGameSetup;
use opts_bevy::SessionEnvironment;

// ---- session ---------------------------------------------------------------------------------

/// Data of the `session` section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionData {
    /// The map's folder below `<OMSI>/maps`.
    pub map_folder: String,
    /// The map's display name.
    pub map_name: String,
    /// The start point chosen for the drive (the drive's entry point name).
    pub entry_point: String,
    /// What kind of session this is (`drive`; later modes get other words).
    pub mode: String,
    /// Seconds of play, summed over every load of this save.
    pub played_s: f64,
}

pub(super) fn session_capture(world: &mut World) -> Result<Option<Value>, String> {
    let map_folder = match world.get_resource::<WorldLoadJob>() {
        Some(job) => job
            .map_dir()
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or("the map folder has no name")?,
        // No loader (a headless test world): the setup's map is the folder.
        None => world.resource::<NewGameSetup>().selected_map.clone(),
    };
    let info = world.resource::<InGameSessionInfo>();
    to_value(&SessionData {
        map_folder,
        map_name: info.map_name.clone(),
        entry_point: info.entry_point.clone(),
        mode: "drive".to_owned(),
        played_s: world
            .get_resource::<PlayClock>()
            .map_or(0.0, |c| c.played_s),
    })
}

pub(super) fn session_prepare(world: &mut World, data: &Value) -> Result<(), String> {
    let d: SessionData = parse(data, "session")?;
    let mut setup = world.resource_mut::<NewGameSetup>();
    setup.selected_map = d.map_folder;
    setup.selected_entry_point = d.entry_point;
    // The vehicles come back through the spawner, not the loading pipeline's first vehicle.
    setup.no_vehicle = true;
    setup.selected_vehicle.clear();
    setup.selected_livery.clear();
    setup.custom_setvars.clear();
    world.resource_mut::<PlayClock>().played_s = d.played_s;
    Ok(())
}

pub(super) fn session_check(data: &Value, env: &CheckEnv) -> Vec<Issue> {
    let Ok(d) = parse::<SessionData>(data, "session") else {
        return Vec::new();
    };
    let missing = |blocking| Issue {
        kind: IssueKind::MissingMap,
        subject: d.map_name.clone(),
        blocking,
    };
    let Some(root) = env.root else {
        return vec![missing(true)];
    };
    let dir = root.join("maps").join(&d.map_folder);
    if !dir.is_dir() {
        return vec![missing(true)];
    }
    match (&env.fingerprint.map, fingerprint::map(&dir)) {
        (Some((_, saved)), Some(now)) if *saved != now => vec![Issue {
            kind: IssueKind::Changed,
            subject: d.map_name,
            blocking: false,
        }],
        _ => Vec::new(),
    }
}
// ---- weather ---------------------------------------------------------------------------------

pub(super) fn weather_capture(world: &mut World) -> Result<Option<Value>, String> {
    // The weather of the session now (the pause page may have changed it), else the plan's.
    let preset = world.get_resource::<SessionEnvironment>().map_or_else(
        || world.resource::<NewGameSetup>().weather_preset.clone(),
        |e| e.weather().to_owned(),
    );
    Ok(Some(json!({ "preset": preset })))
}

pub(super) fn weather_prepare(world: &mut World, data: &Value) -> Result<(), String> {
    let preset = data
        .get("preset")
        .and_then(Value::as_str)
        .ok_or("weather: `preset` is missing")?;
    world.resource_mut::<NewGameSetup>().weather_preset = preset.to_owned();
    Ok(())
}
