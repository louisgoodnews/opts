//! The probe's test scene: two cameras with image targets (black and white) and a few meshes.

use bevy::asset::RenderAssetUsages;
use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::core_pipeline::tonemapping::DebandDither;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::render_resource::{TextureFormat, TextureUsages};

/// Spawns the two cameras (black and white clear colour, each with an image target) and the
/// test scene; returns the two image targets.
pub(super) fn spawn_scene(world: &mut World) -> Vec<Handle<Image>> {
    let mut targets = Vec::new();
    for clear in [Color::BLACK, Color::WHITE] {
        let mut img = Image::new_target_texture(512, 512, TextureFormat::Rgba8UnormSrgb, None);
        img.asset_usage = RenderAssetUsages::RENDER_WORLD;
        img.texture_descriptor.usage |= TextureUsages::COPY_SRC;
        let h = world.resource_mut::<Assets<Image>>().add(img);
        world.spawn((
            Camera3d::default(),
            opts_bevy::MainCamera,
            Camera {
                clear_color: ClearColorConfig::Custom(clear),
                ..default()
            },
            RenderTarget::from(h.clone()),
            Msaa::Off,
            Tonemapping::None,
            DebandDither::Disabled,
            Transform::from_xyz(4.0, 4.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
        targets.push(h);
    }
    let cube = world
        .resource_mut::<Assets<Mesh>>()
        .add(Cuboid::new(2.0, 2.0, 2.0));
    let quad = world
        .resource_mut::<Assets<Mesh>>()
        .add(Rectangle::new(2.0, 2.0));
    let grey = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(Color::srgb(0.6, 0.6, 0.6));
    let magenta = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 1.0),
            unlit: true,
            ..default()
        });
    world.spawn((Mesh3d(cube), MeshMaterial3d(grey)));
    world.spawn((
        Mesh3d(quad),
        MeshMaterial3d(magenta),
        Transform::from_xyz(-2.5, 0.0, 0.0),
    ));
    world.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(-3.0, 5.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    targets
}
