//! Spawning the capture rig: the four cameras, their targets and the light.

use bevy::asset::RenderAssetUsages;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};

use super::*;

/// Brightness of the ambient term (cd/m²): high, so faces turned away from the light stay
/// readable.
const AMBIENT: f32 = 900.0;
/// Illuminance of the directional light (lux).
const SUN_LUX: f32 = 4_000.0;

impl CaptureRig {
    /// Spawns inactive cameras, targets and the light.
    pub fn spawn(world: &mut World) -> Self {
        world.insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: AMBIENT,
            affects_lightmapped_meshes: true,
        });
        let images: [Handle<Image>; CAMERAS] = std::array::from_fn(|_| {
            let mut img = Image::new_target_texture(
                IMAGE_SIZE,
                IMAGE_SIZE,
                TextureFormat::Rgba8UnormSrgb,
                None,
            );
            img.asset_usage = RenderAssetUsages::RENDER_WORLD;
            img.texture_descriptor.usage |= TextureUsages::COPY_SRC;
            world.resource_mut::<Assets<Image>>().add(img)
        });
        let cameras: [Entity; CAMERAS] = std::array::from_fn(|i| {
            let clear = if i % 2 == 0 {
                Color::BLACK
            } else {
                Color::WHITE
            };
            world
                .spawn((
                    Camera3d::default(),
                    opts_bevy::MainCamera,
                    Camera {
                        is_active: false,
                        order: i as isize,
                        clear_color: ClearColorConfig::Custom(clear),
                        ..default()
                    },
                    RenderTarget::from(images[i].clone()),
                    Msaa::Off,
                    Tonemapping::None,
                    DebandDither::Disabled,
                    Transform::default(),
                    Name::new(format!("BenchmarkCamera{i}")),
                ))
                .id()
        });
        let light = world
            .spawn((
                DirectionalLight {
                    illuminance: SUN_LUX,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::default(),
            ))
            .id();
        Self {
            cameras,
            images,
            views: AtomicUsize::new(2),
            light,
            sink: Arc::default(),
        }
    }
}
