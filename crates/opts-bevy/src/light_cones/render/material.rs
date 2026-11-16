//! The light cone material: its style, the uniform it sends and the material that draws the cone.

use super::*;

pub(super) const SHADER: Handle<Shader> =
    bevy::asset::uuid_handle!("6f707473-6c69-6768-7463-6f6e65000001");

/// Look of the cones (live-tunable).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ConeStyle {
    /// Scatter per metre at the centre of a fully lit cone (with the depth fade; the shader halves
    /// it without, where cones also cross walls).
    pub density: f32,
    /// The most one cone adds to a pixel (`0..1`); the accumulation inside a cone is
    /// `1 - exp(-scatter)` and the blend is a screen blend, so overlapping cones never exceed 1.
    pub max_contribution: f32,
    /// Exponent of the falloff from the apex to the base.
    pub axial_power: f32,
    /// Cones whose apex is farther from the camera than this use 4 march steps instead of 8 (m).
    pub far_steps_distance_m: f32,
}

impl Default for ConeStyle {
    fn default() -> Self {
        Self {
            density: 0.035,
            max_contribution: 0.5,
            axial_power: 1.5,
            far_steps_distance_m: 40.0,
        }
    }
}

impl ConeStyle {
    pub(super) fn uniform(self) -> Vec4 {
        Vec4::new(
            self.density,
            self.axial_power,
            self.far_steps_distance_m,
            self.max_contribution,
        )
    }
}

/// The additive cone material (one shared instance; the per-cone data is in the mesh tag).
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct LightConeMaterial {
    #[uniform(0)]
    pub(super) params: Vec4,
}

impl Material for LightConeMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // The shader returns alpha 0: Add shares the premultiplied-alpha pipeline.
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
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Back faces only: the fragment is where the view ray leaves the cone, which also works
        // with the camera inside.
        descriptor.primitive.cull_mode = Some(Face::Front);
        // Screen blend, `src + dst * (1 - src)`, instead of plain addition: many overlapping
        // cones approach white but cannot exceed it (the shader keeps `src` below 1).
        if let Some(fragment) = descriptor.fragment.as_mut() {
            for target in fragment.targets.iter_mut().flatten() {
                let alpha = target.blend.map_or(BlendComponent::REPLACE, |b| b.alpha);
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::OneMinusSrc,
                        operation: BlendOperation::Add,
                    },
                    alpha,
                });
            }
        }
        Ok(())
    }
}
