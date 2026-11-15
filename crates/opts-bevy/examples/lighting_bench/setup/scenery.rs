//! The street scenery: the boxes of the canyon, the ground and road, and the trees.

use super::*;

/// Boxes: ~300, a street canyon along -Z (x = +-9..40), a few lower annexes.
pub(super) fn spawn_boxes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    facade: &[Handle<StandardMaterial>],
    rng: &mut Rng,
) {
    let box_meshes: Vec<Handle<Mesh>> = (0..12)
        .map(|i| {
            let w = 8.0 + (i % 4) as f32 * 2.0;
            let h = 8.0 + (i % 5) as f32 * 4.0;
            let d = 9.0 + (i % 3) as f32 * 2.0;
            meshes.add(Cuboid::new(w, h, d))
        })
        .collect();
    let mut boxes = 0;
    'outer: for bld in 0..24 {
        for side in [-1.0f32, 1.0] {
            for part in 0..7 {
                if boxes >= 300 {
                    break 'outer;
                }
                let mi = (rng.next() * 12.0) as usize % 12;
                let mesh = box_meshes[mi].clone();
                let h = 8.0 + (mi % 5) as f32 * 4.0;
                let w = 8.0 + (mi % 4) as f32 * 2.0;
                let z = -(bld as f32) * 11.0 - 6.0;
                let x = side * (11.0 + (part as f32) * 10.0 * rng.range(0.6, 1.0) + w * 0.5);
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(facade[(bld + part + boxes) % 6].clone()),
                    Transform::from_xyz(x, h * 0.5, z + rng.range(-1.5, 1.5)),
                ));
                boxes += 1;
            }
        }
    }
}

/// Ground and road.
pub(super) fn spawn_road(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) {
    let road = rgba_image(256, |x, y| {
        let n = hash(x, y, 77) * 0.2 + 0.8;
        let line = (x / 4) % 64 == 31 && (y / 16) % 2 == 0;
        let v = if line { 220.0 } else { 70.0 * n };
        [v as u8, v as u8, v as u8, 255]
    });
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(120.0, 330.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color_texture: Some(images.add(road)),
            uv_transform: bevy::math::Affine2::from_scale(Vec2::new(8.0, 40.0)),
            perceptual_roughness: 0.95,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, -150.0),
        NotShadowCaster,
    ));
}

/// Trees: 100 alpha-masked quads (two crossing quads would be 200; use one).
pub(super) fn spawn_trees(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    rng: &mut Rng,
) {
    let tree_mats: Vec<Handle<StandardMaterial>> = (0..2)
        .map(|v| {
            let img = rgba_image(256, |x, y| {
                let (fx, fy) = (x as f32 / 255.0 - 0.5, y as f32 / 255.0);
                let r = ((fx * 1.6).powi(2) + ((fy - 0.38) * 1.2).powi(2)).sqrt();
                let leaf = hash(x / 3, y / 3, v + 3) > 0.25 + r;
                let trunk = fx.abs() < 0.03 && fy > 0.6;
                if (r < 0.45 && leaf) || trunk {
                    let g = 80.0 + hash(x, y, v) * 70.0;
                    [(g * 0.5) as u8, g as u8, (g * 0.4) as u8, 255]
                } else {
                    [0, 0, 0, 0]
                }
            });
            materials.add(StandardMaterial {
                base_color_texture: Some(images.add(img)),
                alpha_mode: AlphaMode::Mask(0.5),
                double_sided: true,
                cull_mode: None,
                perceptual_roughness: 0.9,
                ..default()
            })
        })
        .collect();
    let tree_mesh = meshes.add(Rectangle::new(5.0, 7.0));
    for i in 0..100 {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let z = -4.0 - (i / 2) as f32 * 4.5;
        commands.spawn((
            Mesh3d(tree_mesh.clone()),
            MeshMaterial3d(tree_mats[i % 2].clone()),
            Transform::from_xyz(side * 6.8, 3.5, z).with_rotation(Quat::from_rotation_y(
                rng.range(-0.6, 0.6) + if i % 3 == 0 { 1.2 } else { 0.0 },
            )),
        ));
    }
}
