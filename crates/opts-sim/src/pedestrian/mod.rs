//! Pedestrians: footpath lanes, a lane network, walkers and the gait.
//!
//! Everything here is plain geometry and kinematics (no Bevy): footpaths come from the type-1
//! `[path]` records of spline profiles (`.sli`) and of scenery objects (`.sco`, e.g. crossings),
//! walkers move along them and the gait turns walking speed into joint angles. See
//! `docs/src/simulation/pedestrians.md`.

pub mod gait;
pub mod lane;
pub mod net;

pub use gait::{gait, Gait};
pub use lane::{lanes_from_object, lanes_from_spline, lanes_of_kind, FootLane, Travel};
pub use net::{Advance, End, LaneId, PedestrianNet, Walker};
