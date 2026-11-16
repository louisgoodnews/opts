//! The tree material: the standard material extended by the wind ([`TreeWind`]), and its shader.

use bevy::asset::Asset;
use bevy::pbr::{ExtendedMaterial, MaterialExtension, StandardMaterial};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

pub(super) const SHADER: Handle<Shader> =
    bevy::asset::uuid_handle!("6f707473-7472-6565-7769-6e6400000001");

/// The wind of the tree material.
///
/// Bindless like the `StandardMaterial` it extends: every tree material shares one bind group, so
/// trees of all textures are drawn in a few multi-draw calls. A plain `#[uniform]` made each tree
/// material its own bind group and draw batch (Waldhofen Bahnhof: 5,536 alpha-mask draw calls
/// instead of 7, 285 ms per frame instead of 41). Non-bindless devices read binding 50 as a
/// uniform; bindless ones the index table at 100 and the data array at 101.
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
#[data(50, TreeWindUniform, binding_array(101))]
#[bindless(index_table(range(50..51), binding(100)))]
pub struct TreeWind {
    /// x, y: the direction the wind blows towards (Bevy x and z); z: crown displacement in metres
    /// for a 10 m tree (0: still); w: sway frequency in Hz.
    pub wind: Vec4,
    /// x: 1 for a `[tree]` billboard (the bend follows its texture's height, `1 - v`), 0 for a
    /// tree modelled as a mesh (the bend follows the height above the object alone; its foliage
    /// cards' texture coordinates say nothing about height).
    pub shape: Vec4,
}

/// The GPU data of [`TreeWind`].
#[derive(Clone, Default, ShaderType)]
pub struct TreeWindUniform {
    wind: Vec4,
    shape: Vec4,
}

impl From<&TreeWind> for TreeWindUniform {
    fn from(t: &TreeWind) -> Self {
        Self {
            wind: t.wind,
            shape: t.shape,
        }
    }
}

/// [`TreeWind::shape`] of a `[tree]` billboard.
pub const BILLBOARD: Vec4 = Vec4::new(1.0, 0.0, 0.0, 0.0);
/// [`TreeWind::shape`] of a mesh tree's foliage.
pub const MESH_TREE: Vec4 = Vec4::ZERO;

impl MaterialExtension for TreeWind {
    fn vertex_shader() -> ShaderRef {
        SHADER.into()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        SHADER.into()
    }
}

/// The material of `[tree]` billboards and of mesh trees' foliage.
pub type TreeMaterial = ExtendedMaterial<StandardMaterial, TreeWind>;
