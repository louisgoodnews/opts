//! Bevy asset loaders and entity spawning bridge for OMSI scenery objects (`.sco`)
//! and spline profiles (`.sli`).
//!
//! Bridges parsed scenery assets into Bevy's ECS hierarchy and procedural
//! spline extrusion meshes. The parts live in `scenery/`: the loaders, the centreline poses,
//! the extrusion and the spawning.

use opts_core::formats::map::MapSplineInstance;
use opts_core::formats::scenery::{SceneryObject, SceneryParseError, SplineProfile};

mod extrude;
mod loader;
mod pose;
mod spawn;

pub use extrude::{extrude_spline_profile, extrude_spline_segment};
pub use loader::{
    SceneryLoaderError, SceneryObjectAsset, ScoAssetLoader, SliAssetLoader, SplineProfileAsset,
};
pub use pose::{spline_pose, spline_stations, SplinePose};
pub use spawn::spawn_scenery_object;

#[cfg(test)]
mod tests;
