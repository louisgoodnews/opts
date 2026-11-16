//! Spawned parts of a plan item: mirrors, ballast and the fog volume.

use super::super::cone_material::ConeMaterial;
use super::super::config::Cfg;
use super::super::procedural::sky_cube;
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
