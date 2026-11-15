//! The expected geometry: the triangles OPTS should draw for an object, in the map frame.
//!
//! Meshes are read with OPTS's own parsers and interpreted the way the loader documents it
//! (axis-cycled layout, `.x` frame transforms, `[matl]` overrides, `[alphascale]` materials
//! hidden, keyed meshes decoded exactly). The **placement** (heading
//! clockwise from north, object +Z forward, +X right, +Y up) is computed here independently of
//! the game (`docs/src/architecture/coordinates.md`).

use std::path::Path;

use bevy::math::Vec3;
use opts_bevy::model::find_override;
use opts_bevy::{OmsiInstall, TextureOverride};
use opts_core::formats::o3d::{O3d, VertexLayout};
use opts_core::formats::scenery::{SceneryObject, ScoMesh};

mod collect;
mod meshes;
mod triangles;
mod types;

pub use collect::build;
use meshes::{overrides, read_models};
use triangles::{add_model, tree_billboard};
pub use types::{Expected, ExpectedTri, KeyedSummary, Placement, TriFlags};

#[cfg(test)]
mod tests;
