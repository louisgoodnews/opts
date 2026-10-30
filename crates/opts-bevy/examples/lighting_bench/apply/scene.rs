//! Spawned parts of a plan item: mirrors, ballast, fog volume and cones.

use super::super::cone_material::ConeMaterial;
use super::super::config::Cfg;
use super::super::procedural::{sky_cube, Rng};
use super::super::*;

pub(super) fn spawn_mirrors(
    cfg: &Cfg,
    commands: &mut Commands,
    images: &mut Assets<Image>,
    pos: Vec3,
    look_dir: Vec3,
) {
    let mw = cfg.u("msize", 256);
    for i in 0..cfg.u("mirrors", 0) as usize {
        let mut image = Image::new_fill(
            Extent3d {
                width: mw,
                height: mw / 2,
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
        let mut m = commands.spawn((
            Camera3d::default(),
            Camera {
                order: -1 - i as isize,
                ..default()
            },
            RenderTarget::Image(handle.into()),
            Msaa::Off,
            Transform::from_translation(pos + Vec3::new(if i == 0 { 2.5 } else { -2.5 }, 0.3, 1.0))
                .looking_to(look_dir, Vec3::Y),
            Mirror,
        ));
        if cfg.u("mhdr", 0) == 1 {
            m.insert(Hdr);
        }
        if cfg.u("mibl", 0) == 1 {
            m.insert(GeneratedEnvironmentMapLight {
                environment_map: images.add(sky_cube(64)),
                intensity: 500.0,
                ..default()
            });
        }
    }
}

pub(super) fn spawn_ballast(
    cfg: &Cfg,
    commands: &mut Commands,
    assets: &Assets3,
    cones: &mut Assets<ConeMaterial>,
    pos: Vec3,
    look_dir: Vec3,
) {
    let ballast = cfg.u("ballast", 0);
    if ballast > 0 {
        let mat = cones.add(ConeMaterial {
            params: Vec4::ZERO,
            params2: Vec4::new(ballast as f32, 0.0, 0.0, 0.0),
            march: false,
            ballast: true,
        });
        commands.spawn((
            Mesh3d(assets.quad.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(pos + look_dir * 1.0)
                .looking_to(look_dir, Vec3::Y)
                .with_scale(Vec3::new(1.7, 1.0, 1.0)),
            NotShadowCaster,
            NotShadowReceiver,
            RenderLayers::layer(1),
            bevy::camera::visibility::NoFrustumCulling,
            Cone,
        ));
    }
}

pub(super) fn spawn_fog_volume(cfg: &Cfg, commands: &mut Commands, vspots: usize, pos: Vec3) {
    if cfg.u("fog", 0) == 1 || vspots > 0 {
        let side = cfg.f("vol", 200.0);
        commands.spawn((
            FogVolume {
                density_factor: cfg.f("density", 0.002),
                scattering: 0.3,
                absorption: 0.1,
                scattering_asymmetry: cfg.f("g", 0.6),
                ..default()
            },
            Transform::from_translation(pos).with_scale(Vec3::splat(side)),
            Fog,
        ));
    }
}

pub(super) fn spawn_cones(
    cfg: &Cfg,
    commands: &mut Commands,
    assets: &Assets3,
    cones: &mut Assets<ConeMaterial>,
    pos: Vec3,
    look_dir: Vec3,
) {
    let tech = cfg.s("tech", "none");
    let n = cfg.u("n", 0) as usize;
    if n > 0 && (tech == "mesh" || tech == "march") {
        let march = tech == "march";
        let fade = cfg.u("fade", 0) == 1;
        let mat_l = cones.add(ConeMaterial {
            params: Vec4::new(
                if march {
                    cfg.f("inten", 3.0)
                } else {
                    cfg.f("inten", 0.45)
                },
                cfg.f("edge", 2.0),
                if fade { 1.5 } else { 0.0 },
                cfg.f("axial", 1.5),
            ),
            params2: Vec4::new(cfg.u("msteps", 8) as f32, 0.35, 0.0, cfg.f("dbg", 0.0)),
            march,
            ballast: false,
        });
        if cfg.u("inside", 0) == 1 {
            spawn_inside_cone(commands, assets, &mat_l, pos, look_dir);
        }
        spawn_street_cones(commands, assets, &mat_l, n);
    }
}

fn spawn_inside_cone(
    commands: &mut Commands,
    assets: &Assets3,
    mat_l: &Handle<ConeMaterial>,
    pos: Vec3,
    look_dir: Vec3,
) {
    // A big cone around the camera: the view starts inside its volume.
    let (len, half) = (40.0f32, 22f32.to_radians());
    commands.spawn((
        Mesh3d(assets.cone.clone()),
        MeshMaterial3d(mat_l.clone()),
        Transform::from_translation(pos - look_dir * 3.0 + Vec3::new(0.0, 0.3, 0.0))
            .looking_to(look_dir, Vec3::Y)
            .with_scale(Vec3::new(len * half.tan(), len * half.tan(), len)),
        MeshTag::new(255 | (240 << 8) | (200 << 16)),
        NotShadowCaster,
        NotShadowReceiver,
        Cone,
    ));
}

fn spawn_street_cones(
    commands: &mut Commands,
    assets: &Assets3,
    mat_l: &Handle<ConeMaterial>,
    n: usize,
) {
    let mut rng = Rng(99);
    for i in 0..n {
        let lantern = i % 5 < 3;
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let zf = (i as f32 + 0.5) / n as f32;
        let z = -8.0 - zf * 120.0;
        let (apex, dir, len, half) = if lantern {
            (
                Vec3::new(side * 5.5, 6.5, z),
                Vec3::new(-side * 0.25, -1.0, 0.0).normalize(),
                8.0,
                30f32.to_radians(),
            )
        } else {
            let lane = if rng.next() < 0.5 { -2.0 } else { 2.0 };
            (
                Vec3::new(lane + rng.range(-0.5, 0.5), 0.8, z),
                if lane < 0.0 { Vec3::NEG_Z } else { Vec3::Z },
                35.0,
                14f32.to_radians(),
            )
        };
        let col = if lantern {
            [255u32, 215, 150]
        } else {
            [255, 245, 215]
        };
        let tag = col[0] | (col[1] << 8) | (col[2] << 16);
        commands.spawn((
            Mesh3d(assets.cone.clone()),
            MeshMaterial3d(mat_l.clone()),
            Transform::from_translation(apex)
                .looking_to(dir, Vec3::Y)
                .with_scale(Vec3::new(len * half.tan(), len * half.tan(), len)),
            MeshTag::new(tag),
            NotShadowCaster,
            NotShadowReceiver,
            Cone,
        ));
    }
}
