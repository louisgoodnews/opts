//! Material, picture and camera entity of a mirror: the private copy and its render target.

use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::Tonemapping;

use crate::mirrors::*;

pub(in crate::mirrors) fn mirror_material(
    original: &StandardMaterial,
    picture: Handle<Image>,
    uv_min: Vec2,
    uv_max: Vec2,
    settings: &MirrorSettings,
) -> StandardMaterial {
    let mut m = original.clone();
    m.base_color_texture = Some(picture);
    m.base_color = Color::WHITE;
    m.unlit = true;
    m.alpha_mode = AlphaMode::Opaque;
    m.double_sided = true;
    m.cull_mode = None;
    m.uv_transform = pane_uv_transform(uv_min, uv_max, settings.flip_u, settings.flip_v);
    m
}

pub(super) fn create_mirror_image(size: UVec2) -> Image {
    let mut image = Image::new_target_texture(size.x, size.y, TextureFormat::Bgra8UnormSrgb, None);
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        mag_filter: bevy::image::ImageFilterMode::Linear,
        min_filter: bevy::image::ImageFilterMode::Linear,
        ..default()
    });
    image
}

pub(super) fn spawn_camera_entity(
    picture: Handle<Image>,
    aspect_ratio: f32,
    commands: &mut Commands,
) -> Entity {
    commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: MIRROR_ORDER,
                is_active: false,
                ..default()
            },
            RenderTarget::Image(picture.into()),
            Msaa::Off,
            Hdr,
            Tonemapping::None,
            Projection::Perspective(PerspectiveProjection {
                fov: 40f32.to_radians(),
                near: 0.2,
                aspect_ratio,
                ..default()
            }),
            Transform::default(),
            Name::new("Mirror_Camera"),
        ))
        .id()
}
