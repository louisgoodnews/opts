//! Drawing of scenery light effects: a shared soft glow per light (a disc whose vertex colours
//! fade to nothing at the rim, drawn additively and turned towards the camera), sized by its
//! brightness and faded by the viewing angle. No Bevy light source is created (thousands of
//! signal heads would cost too much).

use std::collections::HashMap;

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use super::effects::SceneryLight;

/// Lights farther than this from the camera are hidden (their glow is a few pixels at most).
pub const LIGHT_DRAW_DISTANCE_M: f32 = 1_500.0;

/// Shared mesh and per-colour materials of light sprites; kept by whoever spawns objects.
#[derive(Debug, Default, Clone)]
pub struct SceneryLightAssets {
    glow: Option<Handle<Mesh>>,
    materials: HashMap<[u8; 3], Handle<StandardMaterial>>,
}

impl SceneryLightAssets {
    /// The unit-diameter glow disc all light sprites share (see [`glow_mesh`]).
    pub fn glow_mesh(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.glow
            .get_or_insert_with(|| meshes.add(glow_mesh()))
            .clone()
    }

    /// The glowing material for colour `rgb` (0–255).
    pub fn material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        rgb: [f32; 3],
    ) -> Handle<StandardMaterial> {
        let key = rgb.map(|c| c.clamp(0.0, 255.0) as u8);
        self.materials
            .entry(key)
            .or_insert_with(|| {
                let c = Color::srgb_u8(key[0], key[1], key[2]);
                // The disc's vertex colours shape the glow; the colour sets its tint and the
                // brightness of the core.
                materials.add(StandardMaterial {
                    base_color: Color::LinearRgba(LinearRgba::from(c) * GLOW_CORE),
                    unlit: true,
                    alpha_mode: AlphaMode::Add,
                    fog_enabled: false,
                    ..default()
                })
            })
            .clone()
    }
}

/// Brightness of a glow's core relative to its colour (additive, so the core saturates to
/// white like a lamp seen at night while the rim fades out).
const GLOW_CORE: f32 = 3.0;

/// Rings of the glow disc: (radius, intensity), centre to rim; the rim is fully transparent so
/// the sprite has no visible edge.
const GLOW_RINGS: [(f32, f32); 4] = [(0.0, 1.0), (0.1, 0.75), (0.25, 0.22), (0.5, 0.0)];

/// A unit-diameter disc in the XY plane facing +Z whose vertex colours fall from 1 at the centre
/// to 0 at the rim (premultiplied: colour and alpha alike), for additive glows.
#[must_use]
pub fn glow_mesh() -> Mesh {
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::{Indices, PrimitiveTopology};
    const SEGMENTS: u32 = 24;
    let mut positions = vec![[0.0, 0.0, 0.0]];
    let mut colors = vec![[1.0, 1.0, 1.0, 1.0]];
    for &(r, k) in &GLOW_RINGS[1..] {
        for i in 0..SEGMENTS {
            let a = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            positions.push([r * a.cos(), r * a.sin(), 0.0]);
            colors.push([k, k, k, k]);
        }
    }
    let ring = |n: u32, i: u32| 1 + n * SEGMENTS + i % SEGMENTS;
    let mut indices = Vec::new();
    for i in 0..SEGMENTS {
        indices.extend([0, ring(0, i), ring(0, i + 1)]);
    }
    for n in 0..(GLOW_RINGS.len() as u32 - 2) {
        for i in 0..SEGMENTS {
            let (a, b, c, d) = (
                ring(n, i),
                ring(n, i + 1),
                ring(n + 1, i),
                ring(n + 1, i + 1),
            );
            indices.extend([a, c, b, b, c, d]);
        }
    }
    let count = positions.len();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; count])
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

/// Spawns the sprite entity of a light (hidden until its first update).
pub fn spawn_light_sprite(
    commands: &mut Commands,
    light: SceneryLight,
    position: Vec3,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
) -> Entity {
    commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(position).with_scale(Vec3::splat(light.size_m)),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
            light,
            Name::new("SceneryLight"),
        ))
        .id()
}

/// Sizes and shows each light sprite by its brightness as seen from the camera, and turns its
/// glow disc towards the camera.
pub fn scenery_light_system(
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut lights: Query<(
        &SceneryLight,
        &GlobalTransform,
        &mut Transform,
        &mut Visibility,
    )>,
) {
    let Some(cam) = cameras
        .iter()
        .find(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation())
    else {
        return;
    };
    for (light, global, mut transform, mut visibility) in &mut lights {
        let pos = global.translation();
        let level = if light.level <= 0.0 || pos.distance(cam) > LIGHT_DRAW_DISTANCE_M {
            0.0
        } else {
            // The direction is in the object's frame; bring the camera into it.
            let to_cam = (cam - pos).normalize_or_zero();
            let local = global.rotation().inverse() * to_cam;
            light.seen_level(local)
        };
        let want = if level > 0.01 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
        if level > 0.01 {
            let scale = Vec3::splat(light.size_m * level.sqrt());
            if transform.scale != scale {
                transform.scale = scale;
            }
            // Face the camera: the world rotation the disc needs, brought into the parent's frame.
            let facing = Quat::from_rotation_arc(Vec3::Z, (cam - pos).normalize_or(Vec3::Z));
            let parent = global.rotation() * transform.rotation.inverse();
            transform.rotation = parent.inverse() * facing;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The glow is brightest at the centre and has no visible edge.
    #[test]
    fn glow_disc_fades_from_centre_to_rim() {
        let mesh = glow_mesh();
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("colours");
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        assert_eq!(colors[0], [1.0; 4]);
        for (p, c) in positions.iter().zip(colors) {
            let r = (p[0] * p[0] + p[1] * p[1]).sqrt();
            if (r - 0.5).abs() < 1e-5 {
                assert_eq!(c[3], 0.0, "rim must be transparent");
            }
            assert!(r <= 0.5 + 1e-5, "unit diameter");
        }
    }
}
