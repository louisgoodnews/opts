//! Tests of the editor's state and actions (no installation needed; spawning is checked live).

use super::state::{Act, Car, FormationCommands, FormationEditor, Summary};
use super::*;
use crate::formation::store::FormationStore;
use crate::formation::FormationLibrary;
use crate::in_game::vehicle_menu::VehicleMenu;
use crate::session_vehicles::SessionVehicles;
use opts_core::formats::trains::Orientation;
use opts_core::formats::trains::{FormationIssue, FormationSource, TrainFormation};
use opts_gui::interaction::events::ButtonClicked;

mod edit;
mod helpers;
mod store;

use helpers::{app, ed, files, say};
