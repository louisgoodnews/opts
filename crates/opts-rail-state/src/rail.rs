//! State of the rail-bound vehicles: the player's rail vehicle and the composed units of a
//! vehicle. The physics live in `opts_sim::rail`.

/// A player vehicle spawned farther than this from any track is not put on rails (m).
const SNAP_MAX_M: f32 = 12.0;
/// Wheel radius assumed for the scripts' torque when the vehicle file states none (m).
const DEFAULT_WHEEL_RADIUS_M: f32 = 0.33;

mod composition;
pub use composition::{Part, VehicleComposition};
mod player;
pub use player::RailPlayer;
