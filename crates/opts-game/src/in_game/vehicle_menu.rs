//! The in-game menu on Left Alt: despawn the player's vehicle, or spawn another one.
//!
//! * **Main page:** the session vehicles (name, repaint, spawn place, active marker) each with
//!   *Switch*, *Respawn* and *Despawn* (the last asks first), then *Spawn vehicle…*, *Close*.
//! * **Jump to entry point:** moves the camera to a chosen entry point of the map (free flight)
//!   and puts the active vehicle, if any, there.
//! * **Select Vehicle** (modal, three columns): the vehicles of the catalogue; the description of
//!   the chosen one with its repaints and a button for its equipment configuration; the entry
//!   points of the current map that no AI vehicle stands on. *Confirm* replaces the player's
//!   vehicle by the chosen one at the chosen entry point, with the chosen repaint and options.
//!
//! The pages are rebuilt whenever the menu state changes; the scroll positions of the lists are
//! carried over. While the menu is open the driving, camera and cockpit inputs are gated
//! (`opts_bevy::MenuModalActive`) and the cursor is free.

use std::collections::HashMap;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::MouseWheel;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use opts_bevy::reporting::{MenuModalActive, ModalActive};
use opts_gui::interaction::events::ButtonClicked;
use opts_gui::prelude::*;

use crate::in_game::omsi_world::{EntryIndex, TrafficVehicle, WorldLoadJob};
use crate::in_game::types::SimulationLoadState;
use crate::new_game_screen::repaints::{vehicle_repaints, VehicleRepaints};
use crate::new_game_screen::setvars_discovery::get_vehicle_setvars;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup, VehicleItem};
use crate::session_vehicles::{
    ops, DespawnReason, EntrySel, SessionVehicles, SpawnParams, VehicleId,
};
use opts_core::setvars::{Respawn, SetvarKind};

/// AI vehicles closer than this to an entry point block it (m).
const ENTRY_BLOCK_RADIUS_M: f32 = 12.0;

mod action;
mod model;
mod plugin;

use action::{Action, ListScroll, MenuRoot};
use model::RowSig;
pub use model::{Page, VehicleMenu};
pub use plugin::{MenuCommands, VehicleMenuPlugin};

mod actions;
use actions::menu_actions;

mod entries;
use entries::{refresh_entries, refresh_session_rows, search_input};

mod rebuild;
use rebuild::rebuild_menu;

// --- building blocks ---------------------------------------------------------------------------

mod build;
mod select_page;

// --- change map (M-0108) ----------------------------------------------------------------------

mod change_map_page;
use change_map_page::{confirm as confirm_change_map, ChangeMapDraft};

// --- configurations (M-0077) -----------------------------------------------------------------

mod configs_page;
use configs_page::{load_view, ConfigsView};

// --- the Setvars page (M-0110) ---------------------------------------------------------------

mod setvars_page;
use setvars_page::LiveSetvars;

// --- the route page (M-0099) ------------------------------------------------------------------

mod route_page;
use route_page::scroll_lists;

#[cfg(test)]
mod route_page_tests;
