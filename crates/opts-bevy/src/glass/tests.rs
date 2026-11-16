//! Unit tests of `glass.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

fn blended() -> StandardMaterial {
    StandardMaterial {
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.8,
        reflectance: 0.5,
        base_color: Color::srgba(0.8, 0.9, 1.0, 0.4),
        ..default()
    }
}

fn test_app() -> App {
    let mut app = App::new();
    app.init_resource::<Assets<StandardMaterial>>()
        .insert_resource(LightQuality::default())
        .init_resource::<GlassStats>()
        .add_systems(Update, sync_glass);
    app
}
fn window_image(rgba: [u8; 4]) -> Image {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let data: Vec<u8> = (0..16).flat_map(|_| rgba).collect();
    Image::new(
        Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

mod pixels;
mod setting;
mod texture;
