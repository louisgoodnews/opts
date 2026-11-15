//! AI trains and bikes on the rail network: the unit and train specs read from the vehicle files, the variants, spawning and moving them.

use super::obstacles::{nearest_gap, Obstacle};
use super::spec::{load_spec, load_zug_spec, TrainSpec};
use super::*;

mod step;
pub use step::move_ai_trains;

mod spawn;
mod train;
mod types;
mod variants;

pub use spawn::spawn_ai_trains;
pub(super) use train::build_train;
pub use types::*;
pub use variants::load_rail_variants;
