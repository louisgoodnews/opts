//! Scenery object (`.sco`) and spline profile (`.sli`) parsers and data models.

pub mod error;
pub mod sco;
pub mod sli;

pub use error::SceneryParseError;
pub use sco::{
    SceneryObject, ScoAnimation, ScoBoundingBox, ScoCtc, ScoMaterial, ScoMesh, ScoPath, ScoPhase,
    ScoSplineHelper, ScoTextTexture, ScoTrafficLightGroup, ScoTrafficLightProgram, ScoTree,
};
pub use sli::{
    LaneKind, SplineGraphicalProfile, SplineHeightProfile, SplinePath, SplineProfile,
    SplineProfilePoint, SplineRailEnh, SplineTexture,
};
