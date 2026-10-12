//! Drawing of scenery light effects: a shared glowing sphere per light, sized by its brightness
//! and faded by the viewing angle. No Bevy light source is created (thousands of signal heads
//! would cost too much).

use std::collections::HashMap;

use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;

use super::effects::SceneryLight;

/// Lights farther than this from the camera are hidden (their glow is a few pixels at most).
pub const LIGHT_DRAW_DISTANCE_M: f32 = 1_500.0;

/// Shared mesh and per-colour materials of light sprites; kept by whoever spawns objects.
#[derive(Debug, Default, Clone)]
pub struct SceneryLightAssets {
    sphere: Option<Handle<Mesh>>,
    materials: HashMap<[u8; 3], Handle<StandardMaterial>>,
}

impl SceneryLightAssets {
    /// The unit-diameter sphere all light sprites share.
    pub fn sphere(&mut self, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
        self.sphere
            .get_or_insert_with(|| meshes.add(Sphere::new(0.5).mesh().uv(12, 8)))
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
                materials.add(StandardMaterial {
                    base_color: c,
                    emissive: LinearRgba::from(c) * 4.0,
                    unlit: true,
                    alpha_mode: AlphaMode::Add,
                    fog_enabled: false,
                    ..default()
                })
            })
            .clone()
    }
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

/// Sizes and shows each light sprite by its brightness as seen from the camera.
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
        }
    }
}
