//! Footpath lanes sampled from spline profiles and scenery-object paths.

use opts_core::formats::map::MapSplineInstance;
use opts_core::formats::scenery::sco::ScoPath;
use opts_core::formats::scenery::{LaneKind, SplineProfile};

mod model;
mod object;
mod spline;

pub use model::{FootLane, Gate, Travel};
pub use object::{
    lanes_from_object, lanes_from_object_of_type, lanes_from_points_object, SCO_STRAIGHT_RADIUS,
};
pub use spline::{lanes_from_spline, lanes_of_kind};

/// Path type of pedestrians in `[path]` records.
pub const PATH_TYPE_PEDESTRIAN: u32 = 1;
/// Spacing of the lane's polyline vertices (m).
const SAMPLE_STEP_M: f32 = 1.0;

#[cfg(test)]
mod tests;
