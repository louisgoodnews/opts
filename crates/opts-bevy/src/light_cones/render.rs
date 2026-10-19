//! Drawing the cones: style, material, shared assets, the unit cone mesh and the render plugin.

use super::*;

const SHADER: Handle<Shader> = bevy::asset::uuid_handle!("6f707473-6c69-6768-7463-6f6e65000001");

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
    fn uniform(self) -> Vec4 {
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

/// The shared cone mesh and material.
#[derive(Resource, Debug, Clone)]
pub struct ConeAssets {
    /// The unit cone.
    pub mesh: Handle<Mesh>,
    /// The cone material.
    pub material: Handle<LightConeMaterial>,
}

impl FromWorld for ConeAssets {
    fn from_world(world: &mut World) -> Self {
        let style = world
            .get_resource::<ConeStyle>()
            .copied()
            .unwrap_or_default();
        let mesh = world.resource_mut::<Assets<Mesh>>().add(unit_cone_mesh());
        let material = world
            .resource_mut::<Assets<LightConeMaterial>>()
            .add(LightConeMaterial {
                params: style.uniform(),
            });
        Self { mesh, material }
    }
}

/// Unit cone: apex at the origin, base ring (radius 1) at z = -1, axis -Z, plus a base cap.
#[must_use]
pub fn unit_cone_mesh() -> Mesh {
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
    // The CPU copy stays: the bounding box for frustum culling is computed from it.
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(pos),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(nor),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float32x2(uv))
    .with_inserted_indices(Indices::U32(idx))
}

/// Gives new cones their mesh and material.
fn attach_cone_assets(
    mut commands: Commands,
    assets: Res<ConeAssets>,
    fresh: Query<Entity, (With<LightCone>, Without<Mesh3d>)>,
) {
    for e in &fresh {
        commands.entity(e).insert((
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
        ));
    }
}

/// Pushes a changed [`ConeStyle`] into the shared material.
fn sync_cone_style(
    style: Res<ConeStyle>,
    assets: Res<ConeAssets>,
    mut materials: ResMut<Assets<LightConeMaterial>>,
) {
    if !style.is_changed() {
        return;
    }
    if let Some(mut m) = materials.get_mut(&assets.material) {
        m.params = style.uniform();
    }
}

/// The cone material, its shader and the systems that give cones their mesh. Needs the render
/// app; the game adds it next to the ground material.
#[derive(Default)]
pub struct LightConeRenderPlugin;

impl Plugin for LightConeRenderPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SHADER, "../shaders/light_cone.wesl", Shader::from_wesl);
        if !app.is_plugin_added::<LightConePlugin>() {
            app.add_plugins(LightConePlugin);
        }
        app.add_plugins(MaterialPlugin::<LightConeMaterial>::default())
            .init_resource::<ConeStyle>()
            .init_resource::<ConeAssets>()
            .add_systems(Update, (attach_cone_assets, sync_cone_style));
    }
}
