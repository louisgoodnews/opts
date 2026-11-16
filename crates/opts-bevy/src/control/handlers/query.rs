//! `state`, `camera.get`, `sim.get`, `log.tail`, `log.filter`.

use bevy::ecs::world::World;
use bevy::prelude::*;
use bevy::time::{Real, Time, Virtual};
use opts_cli::protocol::RpcError;
use opts_logging::{LogQuery, LogRecord};
use serde_json::{json, Map, Value};

use crate::camera::CameraState;
use crate::control::params::{opt_str, opt_u64};
use crate::control::state::{ControlRegistry, ControlState, Outcome};

/// Registers the query methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("state", state);
    r.register("camera.get", camera_get);
    r.register("sim.get", sim_get);
    r.register("log.tail", log_tail);
    r.register("log.filter", log_filter);
}
mod env;
mod logs;
mod state;

pub use state::*;
use env::*;
use logs::*;

#[cfg(test)]
mod tests;
