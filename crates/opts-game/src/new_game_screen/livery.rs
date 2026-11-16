//! Liveries and equipment options of the chosen vehicle, discovered off the main thread.

use bevy::prelude::*;
use bevy::tasks::{futures::check_ready, IoTaskPool, Task};
use opts_gui::prelude::*;

use super::choice::{spawn_choice, ChoiceStep};
use super::planner::PlannerStep;
use super::repaints::{vehicle_repaints, VehicleRepaints};
use super::setvars_discovery::setvars_from;
use super::types::{GameCatalogue, NewGameSetup, VehicleItem};
use super::ui::{action_button, column, keyed, keyed_args, plain, row, Emphasis, PlannerAction};
mod fill;
mod section;
mod state;
pub use fill::{fill_liveries, fill_options};
pub use section::spawn_livery_section;
pub use state::{poll_liveries, request, LiveryBox, LiveryState, LiverySwatch, OptionsBox};
