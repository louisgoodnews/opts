//! Spawned street and inside cones of a plan item.

use super::super::cone_material::ConeMaterial;
use super::super::config::Cfg;
use super::super::procedural::Rng;
use super::super::*;

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
