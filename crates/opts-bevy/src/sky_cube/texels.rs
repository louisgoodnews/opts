//! The texels of the sky cube: the sky's radiance per direction, the cube's face layout, and the
//! image that holds them.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

use super::input::SkyCubeInput;
use super::tuning::{GROUND_SHARE, SUN_SPOT_POWER, SUN_SPOT_RADIANCE};

/// The radiance the sky has in direction `d` (unit).
#[must_use]
pub fn sky_radiance(d: Vec3, input: &SkyCubeInput) -> Vec3 {
    let horizon = Vec3::from(input.horizon);
    let zenith = Vec3::from(input.zenith);
    let up = d.y;
    let mut c = if up >= 0.0 {
        horizon.lerp(zenith, up.powf(0.5))
    } else {
        horizon.lerp(horizon * GROUND_SHARE, (-up * 6.0).min(1.0))
    };
    if input.sun_elevation_deg > 0.0 {
        c += Vec3::from(input.sun_color)
            * (SUN_SPOT_RADIANCE * input.direct)
            * d.dot(input.sun_dir).max(0.0).powf(SUN_SPOT_POWER);
    }
    c
}

/// Direction of the texel `(x, y)` on cube face `face` (Bevy's face order +X -X +Y -Y +Z -Z).
#[must_use]
pub fn face_direction(face: u32, x: u32, y: u32, size: u32) -> Vec3 {
    let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
    let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
    match face {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    }
    .normalize()
}

/// The texel data (`Rgba16Float`, six faces after each other) of the cube for `input`.
#[must_use]
pub fn cube_data(input: &SkyCubeInput, size: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((size * size * 6 * 8) as usize);
    for face in 0..6 {
        for y in 0..size {
            for x in 0..size {
                let c = sky_radiance(face_direction(face, x, y, size), input);
                for ch in [c.x, c.y, c.z, 1.0] {
                    data.extend_from_slice(&crate::reflections::f16_bits(ch).to_le_bytes());
                }
            }
        }
    }
    data
}

pub(super) fn cube_image(input: &SkyCubeInput, size: u32) -> Image {
    let mut image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 6,
        },
        TextureDimension::D2,
        cube_data(input, size),
        TextureFormat::Rgba16Float,
        // The main world keeps the data so the cube can be rewritten in place.
        RenderAssetUsages::default(),
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    image
}
