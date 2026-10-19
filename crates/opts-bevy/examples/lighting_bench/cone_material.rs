//! The light-cone material and mesh prototype.

use super::*;

pub(super) const SHADER: Handle<Shader> =
    bevy::asset::uuid_handle!("6c696768-7462-656e-6368-636f6e657300");

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct ConeKey {
    pub(super) march: bool,
    pub(super) ballast: bool,
}

/// Additive cone material (mesh or ray-marched, see the shader).
#[derive(Asset, AsBindGroup, TypePath, Clone)]
#[bind_group_data(ConeKey)]
pub(super) struct ConeMaterial {
    #[uniform(0)]
    pub(super) params: Vec4,
    #[uniform(1)]
    pub(super) params2: Vec4,
    pub(super) march: bool,
    pub(super) ballast: bool,
}

impl From<&ConeMaterial> for ConeKey {
    fn from(m: &ConeMaterial) -> Self {
        Self {
            march: m.march,
            ballast: m.ballast,
        }
    }
}

impl Material for ConeMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if key.bind_group_data.ballast {
            descriptor.primitive.cull_mode = None;
            if let Some(f) = descriptor.fragment.as_mut() {
                f.shader_defs.push("BALLAST".into());
            }
        } else if key.bind_group_data.march {
            descriptor.primitive.cull_mode = Some(Face::Front);
            if let Some(f) = descriptor.fragment.as_mut() {
                f.shader_defs.push("CONE_MARCH".into());
            }
        } else {
            descriptor.primitive.cull_mode = None;
        }
        Ok(())
    }
}

/// Unit cone: apex at the origin, base ring (radius 1) at z = -1, axis -Z, plus a base cap.
pub(super) fn cone_mesh() -> Mesh {
    const SEG: usize = 24;
    let mut pos: Vec<[f32; 3]> = Vec::new();
    let mut nor: Vec<[f32; 3]> = Vec::new();
    let mut uv: Vec<[f32; 2]> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    // Outward side normals of a cone of half-angle atan(1): (cos, sin, 1) normalised.
    for i in 0..SEG {
        let a0 = i as f32 / SEG as f32 * std::f32::consts::TAU;
        let a1 = (i + 1) as f32 / SEG as f32 * std::f32::consts::TAU;
        let am = 0.5 * (a0 + a1);
        let n = |a: f32| Vec3::new(a.cos(), a.sin(), 1.0).normalize().to_array();
        let base = pos.len() as u32;
        pos.push([0.0, 0.0, 0.0]);
        nor.push(n(am));
        uv.push([0.5, 0.0]);
        pos.push([a0.cos(), a0.sin(), -1.0]);
        nor.push(n(a0));
        uv.push([0.0, 1.0]);
        pos.push([a1.cos(), a1.sin(), -1.0]);
        nor.push(n(a1));
        uv.push([1.0, 1.0]);
        // Counter-clockwise seen from outside.
        idx.extend_from_slice(&[base, base + 1, base + 2]);
    }
    // Cap.
    let c = pos.len() as u32;
    pos.push([0.0, 0.0, -1.0]);
    nor.push([0.0, 0.0, -1.0]);
    uv.push([0.5, 0.5]);
    for i in 0..=SEG {
        let a = i as f32 / SEG as f32 * std::f32::consts::TAU;
        pos.push([a.cos(), a.sin(), -1.0]);
        nor.push([0.0, 0.0, -1.0]);
        uv.push([0.5, 0.5]);
    }
    for i in 0..SEG as u32 {
        idx.extend_from_slice(&[c, c + 2 + i, c + 1 + i]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, nor)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(idx))
}
