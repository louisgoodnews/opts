//! The camera's own settings of an applied plan item: transform, MSAA, HDR, bloom, environment and the fog prepass.

use super::super::procedural::sky_cube;
use super::*;

pub(super) fn apply_camera(
    cfg: &Cfg,
    commands: &mut Commands,
    cam: Entity,
    images: &mut Assets<Image>,
    pos: Vec3,
    look_dir: Vec3,
) {
    commands
        .entity(cam)
        .insert(Transform::from_translation(pos).looking_to(look_dir, Vec3::Y));
    if cfg.u("msaa", 4) == 0 {
        commands.entity(cam).insert(Msaa::Off);
    } else if cfg.u("msaa", 4) == 2 {
        commands.entity(cam).insert(Msaa::Sample2);
    } else {
        commands.entity(cam).insert(Msaa::Sample4);
    }
    if cfg.u("hdr", 0) == 1 {
        commands.entity(cam).insert(Hdr);
    } else {
        commands.entity(cam).remove::<Hdr>();
    }
    if cfg.u("bloom", 0) == 1 {
        let mut b = Bloom::NATURAL;
        b.intensity = cfg.f("bloomi", 0.05);
        commands.entity(cam).insert(b);
    } else {
        commands.entity(cam).remove::<Bloom>();
    }
    if cfg.u("ibl", 0) == 1 {
        commands.entity(cam).insert(GeneratedEnvironmentMapLight {
            environment_map: images.add(sky_cube(64)),
            intensity: 500.0,
            ..default()
        });
    } else {
        commands
            .entity(cam)
            .remove::<GeneratedEnvironmentMapLight>();
    }
}

pub(super) fn apply_prepass_fog(cfg: &Cfg, commands: &mut Commands, cam: Entity, vspots: usize) {
    let need_prepass = cfg.u("prepass", 0) == 1 || cfg.u("fade", 0) == 1;
    if need_prepass {
        commands.entity(cam).insert(DepthPrepass);
    } else {
        commands.entity(cam).remove::<DepthPrepass>();
    }
    if cfg.u("fog", 0) == 1 || vspots > 0 {
        commands.entity(cam).insert(VolumetricFog {
            step_count: cfg.u("steps", 64),
            jitter: cfg.f("jitter", 0.0),
            ambient_intensity: cfg.f("ambient", 0.0),
            ambient_color: Color::WHITE,
        });
    } else {
        commands.entity(cam).remove::<VolumetricFog>();
    }
}
