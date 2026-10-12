//! Scenery object (`.sco`) and spline profile (`.sli`) parsers and data models.

pub mod error;
pub mod light;
pub mod sco;
pub mod sli;

pub use error::SceneryParseError;
pub use light::ScoLight;
pub use sco::{
    SceneryObject, ScoBoundingBox, ScoCtc, ScoMaterial, ScoMesh, ScoPath, ScoPhase,
    ScoSplineHelper, ScoTextTexture, ScoTrafficLightGroup, ScoTrafficLightProgram, ScoTree,
};
pub use sli::{
    SplineGraphicalProfile, SplineHeightProfile, SplinePath, SplineProfile, SplineProfilePoint,
    SplineRailEnh, SplineTexture,
};
