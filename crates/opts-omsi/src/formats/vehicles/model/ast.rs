//! Abstract Syntax Tree (AST) data models for vehicle model configurations (`model.cfg`).

mod animation;
mod config;
mod light;
mod material;
mod mesh;
mod smoke;
mod texture;
mod viewpoint;

pub use animation::{AnimAction, AnimType, ModelAnim};
pub use config::ModelConfig;
pub use light::{InteriorLight, Spotlight};
pub use material::{
    BumpMap, EnvMap, FreeTex, LightMap, MaterialModifier, ModelMaterial, TextureChange,
};
pub use mesh::{MeshBone, MeshVisibility, ModelLod, ModelMesh};
pub use smoke::SmokeEmitter;
pub use texture::{CtcScheme, CtcTexture, ScriptTexture, TextTexture};
pub use viewpoint::ViewpointFlags;
