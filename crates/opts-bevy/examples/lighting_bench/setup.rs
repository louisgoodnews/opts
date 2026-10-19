//! Scene setup.

use super::cone_material::cone_mesh;
use super::procedural::{hash, rgba_image, Rng};
use super::*;

pub(super) fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    win: Res<WindowModeFlag>,
) {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let cone_handle = meshes.add(cone_mesh());

    // Facade textures: brick-ish with window grid; 6 variants.
    let facade: Vec<Handle<StandardMaterial>> = (0..6)
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
        .collect();

    // Boxes: ~300, a street canyon along -Z (x = +-9..40), a few lower annexes.
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

    // Ground and road.
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

    // Trees: 100 alpha-masked quads (two crossing quads would be 200; use one).
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

    // Street lamps: 48 map point lights (as MapLightPool: r = 4.5 m, 10 klx at r).
    let lumens = 4.0 * std::f32::consts::PI * 4.5 * 4.5 * 10_000.0;
    for i in 0..48 {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        commands.spawn((
            PointLight {
                intensity: lumens,
                range: 36.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(side * 5.5, 6.5, -4.0 - (i / 2) as f32 * 9.0),
            Visibility::Hidden,
            NightLamp(i),
        ));
    }
    // Headlight spots (vehicle_glow.rs: 60 klm, 60 m, 12 / 30 deg).
    for i in 0..8 {
        let lane = if i % 2 == 0 { -2.0 } else { 2.0 };
        let (z, dir) = if i % 2 == 0 {
            (-12.0 - i as f32 * 14.0, Vec3::NEG_Z)
        } else {
            (-45.0 - i as f32 * 10.0, Vec3::Z)
        };
        commands.spawn((
            SpotLight {
                intensity: 60_000.0,
                range: 60.0,
                inner_angle: 12f32.to_radians(),
                outer_angle: 30f32.to_radians(),
                shadow_maps_enabled: false,
                color: Color::srgb(1.0, 0.95, 0.85),
                ..default()
            },
            Transform::from_xyz(lane, 0.8, z).looking_to(dir, Vec3::Y),
            Visibility::Hidden,
            HeadSpot(i),
        ));
    }

    // Sun (SunLight in sky.rs: default DirectionalLight with shadows, default cascades).
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 20_000.0,
            ..default()
        },
        Transform::default().looking_to(Vec3::NEG_Y, Vec3::Z),
        Sun,
    ));

    // Camera.
    let mut cam = commands.spawn((
        Camera3d::default(),
        opts_bevy::MainCamera,
        Transform::from_xyz(0.0, 1.8, 6.0).looking_to(Vec3::NEG_Z, Vec3::Y),
        Tonemapping::default(),
        Msaa::Sample4,
        RenderLayers::from_layers(&[0, 1]),
        Cam,
    ));
    let mut target = None;
    if !win.0 {
        let mut image = Image::new_fill(
            Extent3d {
                width: W,
                height: H,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC
            | TextureUsages::RENDER_ATTACHMENT;
        let handle = images.add(image);
        cam.insert(RenderTarget::Image(handle.clone().into()));
        target = Some(handle);
    }
    let _ = RenderLayers::layer(0);
    commands.insert_resource(Assets3 {
        cone: cone_handle,
        target,
        quad: meshes.add(Rectangle::new(1.0, 1.0)),
    });
}
