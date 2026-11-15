//! The facade textures of the street: the brick-like and window-grid variants.

use super::*;

/// Facade textures: brick-ish with window grid; 6 variants.
pub(super) fn facade_materials(
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Vec<Handle<StandardMaterial>> {
    (0..6)
        .map(|v| {
            let tint = [
                [200u8, 160, 140],
                [190, 190, 175],
                [170, 140, 120],
                [210, 200, 170],
                [160, 165, 175],
                [185, 150, 130],
            ][v as usize];
            let img = rgba_image(512, |x, y| {
                let brick = ((x / 16 + (y / 8) % 2 * 8) % 32 < 1 || y % 8 == 0) as u32;
                let win_cell =
                    (x % 128 > 36 && x % 128 < 92 && y % 128 > 30 && y % 128 < 96) as u32;
                let n = hash(x, y, v) * 0.15 + 0.85;
                let base = if win_cell == 1 {
                    [60.0, 80.0, 105.0]
                } else if brick == 1 {
                    [120.0, 115.0, 110.0]
                } else {
                    [tint[0] as f32, tint[1] as f32, tint[2] as f32]
                };
                [
                    (base[0] * n) as u8,
                    (base[1] * n) as u8,
                    (base[2] * n) as u8,
                    255,
                ]
            });
            materials.add(StandardMaterial {
                base_color_texture: Some(images.add(img)),
                perceptual_roughness: 0.9,
                ..default()
            })
        })
        .collect()
}
