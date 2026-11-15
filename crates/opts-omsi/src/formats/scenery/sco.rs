//! Parser and data structures for OMSI Scenery Object (`.sco`) files.
//!
//! A `.sco` file defines a scenery object (building, tree, street lamp, junction,
//! traffic light, etc.) placed on map tiles. It specifies 3D mesh attachments,
//! materials, collision geometry, traffic paths, lights, and scripting hooks.
//!
//! See specification in `docs/src/formats/text/sco.md`.

use glam::Vec3;

use super::error::SceneryParseError;
use super::light::{parse_light_enh, parse_light_enh_2, ScoLight};
use crate::formats::text::{decode_bytes, parse_float, scan_blocks, OmsiBlock};
use crate::formats::vehicles::model::{parse_newanim, ModelAnim};

mod data;
mod material;
mod meshes;
mod object;
mod record;
mod signals;
mod texts;
mod traffic;

pub use data::{
    ScoAttachment, ScoBoundingBox, ScoCtc, ScoMapLight, ScoPath, ScoSplineHelper, ScoTextTexture,
    ScoTree,
};
pub use material::{ScoMaterial, ScoMesh};
pub use record::SceneryObject;
pub use signals::{ScoPhase, ScoTrafficLightGroup, ScoTrafficLightProgram};

mod parse;
use parse::*;

#[cfg(test)]
mod tests;
