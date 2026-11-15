//! The route of a vehicle (M-0099): its depot (`.hof`), the line and course the driver has
//! selected, the stop it is at, and the keypad sequence that makes the vehicle's own IBIS accept
//! a selection.
//!
//! The data is a service of its own (strategy decision D7): [`RouteHandle`] is shared between
//! the callbacks the scripts call (`GetRouteIndex`, `GetTT*`, ...), the control protocol and the
//! game, so a change of the selection is seen by the very next script call. The scripts own the
//! displays; the host only hands them data and the driver's inputs.

use std::path::Path;
use std::sync::{Arc, RwLock};

use opts_core::formats::hof::HofFile;
use opts_core::formats::timetable::{ActiveSchedule, Timetables};
use opts_sim::script::ScriptEngine;

mod depot;
mod handle;
mod keys;
mod queries;
mod scripts;
mod select;
mod state;
mod systems;
pub use depot::*;
pub use keys::{has_drucker_keypad, has_ibis_keypad, ibis_keys, KeyQueue};
pub use scripts::*;
pub use select::*;
pub use state::*;
pub use systems::*;

/// A shared route state; clones refer to the same state.
#[derive(Debug, Clone, Default)]
pub struct RouteHandle(Arc<RwLock<RouteState>>);
