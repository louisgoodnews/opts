//! The precipitation's textures, materials and mesh, built once.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use opts_core::formats::weather::PrecipKind;

use super::geometry::seed_of;
use super::state::PrecipState;

pub(in super::super) fn texture(width: u32, height: u32, alpha: impl Fn(f32, f32) -> f32) -> Image {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let a = alpha(
                (x as f32 + 0.5) / width as f32,
                (y as f32 + 0.5) / height as f32,
            );
            data.extend([255, 255, 255, (a.clamp(0.0, 1.0) * 255.0) as u8]);
        }
    }
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn material(image: Handle<Image>, colour: Color) -> StandardMaterial {
    StandardMaterial {
        base_color: colour,
        base_color_texture: Some(image),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        cull_mode: None,
        ..default()
    }
}

/// Colour of the particles for a scene-linear brightness `k` (`WeatherLook::precip`).
pub(super) fn base_colour(kind: PrecipKind, k: f32) -> Color {
    match kind {
        PrecipKind::Snow => Color::linear_rgba(k, k, k, 0.9),
        _ => Color::linear_rgba(0.85 * k, 0.9 * k, k, 0.45),
    }
}

/// A mesh with vertices for `quads` quads (positions are written every frame) and its indices.
pub(super) fn quad_mesh(quads: usize) -> (Mesh, Vec<u32>) {
    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices: Vec<u32> = (0..quads as u32)
        .flat_map(|q| [0, 1, 2, 0, 2, 3].map(|i| q * 4 + i))
        .collect();
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32; 3]; quads * 4])
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; quads * 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        (0..quads).flat_map(|_| uv).collect::<Vec<[f32; 2]>>(),
    )
    .with_inserted_indices(Indices::U32(indices[..0].to_vec()));
    (mesh, indices)
}

/// Materials and mesh, once; a changed budget builds the mesh again.
pub(super) fn ensure_assets(
    state: &mut PrecipState,
    (images, meshes, materials): (
        &mut Assets<Image>,
        &mut Assets<Mesh>,
        &mut Assets<StandardMaterial>,
    ),
    budget: usize,
) {
    if state.rain.is_none() {
        let streak = images.add(texture(4, 32, |u, v| {
            (1.0 - (2.0 * u - 1.0).abs()).powf(0.7) * (std::f32::consts::PI * v).sin().powf(1.4)
        }));
        let flake = images.add(texture(16, 16, |u, v| {
            let d = ((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * 2.0;
            (1.0 - d).clamp(0.0, 1.0).powf(0.6)
        }));
        state.rain = Some(materials.add(material(streak, base_colour(PrecipKind::Rain, 1.0))));
        state.snow = Some(materials.add(material(flake, base_colour(PrecipKind::Snow, 1.0))));
    }
    if state.mesh.is_none() || state.allocated != budget {
        let (mesh, indices) = quad_mesh(budget);
        match state.mesh.clone() {
            Some(handle) => {
                let _ = meshes.insert(&handle, mesh);
            }
            None => state.mesh = Some(meshes.add(mesh)),
        }
        state.seeds = (0..budget as u32).map(seed_of).collect();
        state.indices = indices;
        state.allocated = budget;
        state.drawn = 0;
    }
}
