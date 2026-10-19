//! Vehicle model configuration parser (`model.cfg`).
//!
//! OMSI vehicle model configurations describe the 3D rendering manifest of a bus
//! or tram, including mesh geometry references (`.o3d`), level-of-detail (LOD)
//! switching distance tiers, viewpoint camera bitmasks, material slots and modifiers,
//! dynamic text textures, interior lights, headlights, and variable-driven animations.
//!
//! See [`model_cfg.md`](../../../../../../docs/src/formats/text/vehicles/model_cfg.md) for complete specification.

pub mod anim;
pub mod ast;
pub mod error;
pub mod parse;

pub use anim::parse_newanim;

pub use ast::{
    AnimAction, AnimType, BumpMap, CtcScheme, CtcTexture, EnvMap, FreeTex, InteriorLight, LightMap,
    MaterialModifier, MeshBone, MeshVisibility, ModelAnim, ModelConfig, ModelLod, ModelMaterial,
    ModelMesh, ScriptTexture, Spotlight, TextTexture, TextureChange, ViewpointFlags,
};
pub use error::ModelParseError;
pub use parse::parse_model;

use crate::formats::text::decode_bytes;

impl ModelConfig {
    /// Parses a vehicle model configuration from a string slice.
    ///
    /// # Errors
    ///
    /// Returns [`ModelParseError`] if required parameters are missing or numerical
    /// values are malformed.
    pub fn parse(text: &str) -> Result<Self, ModelParseError> {
        parse_model(text)
    }

    /// Parses a vehicle model configuration from raw bytes, falling back to
    /// Windows-1252 (CP1252) if input is not valid UTF-8.
    ///
    /// # Errors
    ///
    /// Returns [`ModelParseError`] if required parameters are missing or numerical
    /// values are malformed.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, ModelParseError> {
        let text = decode_bytes(bytes);
        parse_model(&text)
    }
}
