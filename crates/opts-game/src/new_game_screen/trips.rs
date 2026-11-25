//! The optional step *Trip* (M-0145): the timetable runs the player may drive with the chosen map,
//! bus and clock. Choosing one replaces the start point by the run's first stop; the drive then
//! follows that run.
//!
//! The runs come from the map's `TTData` (line files marked `[userallowed]`), read once per map
//! off the main thread. A run is offered when it departs within [`WINDOW_S`] after the chosen
//! clock on the chosen weekday and its first stop is a bus stop of `Busstops.cfg`.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use bevy::tasks::{futures::check_ready, IoTaskPool, Task};
use opts_core::formats::timetable::Timetables;
use opts_gui::patterns::{
    glyphs, spawn_master_detail, ListEntry, MasterDetail, MasterDetailConfig,
};
use opts_gui::prelude::*;

use super::planner::{PlannerState, PlannerStep};
use super::step_where::StepList;
use super::types::{GameCatalogue, NewGameSetup};
use super::ui::{column, keyed, keyed_args, plain};
use super::BuildCtx;
use crate::in_game::omsi_world::line_runs_on;

/// How long after the chosen clock a run may depart to be offered (s).
pub const WINDOW_S: u32 = 3_600;

/// Id of the *No trip* entry.
pub const NO_TRIP_ID: &str = "__no_trip__";

use opts_new_game_types::trip_choice as choice;
mod list;
mod refresh;
pub use choice::{TripChoice, TripOption};
pub use list::{list_trips, tour_runs_on};
pub use refresh::{refresh_trips, TripState};

mod ui;
pub use ui::{fill_no_trip_detail, fill_trip_detail, spawn_trip};

#[cfg(test)]
mod tests;
