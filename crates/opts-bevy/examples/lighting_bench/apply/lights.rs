//! The sun, the night lamps and the spot lights of an applied plan item.

use super::*;

pub(super) fn apply_sun(
    cfg: &Cfg,
    commands: &mut Commands,
    sun: &mut SunQ,
    night: bool,
    moon: bool,
    toward: bool,
) {
    let elev = cfg.f("elev", 10.0).to_radians();
    let az = 35f32.to_radians();
    let sz = if toward { -az.cos() } else { az.cos() };
    let sun_dir = Vec3::new(az.sin() * elev.cos(), elev.sin(), sz * elev.cos()).normalize();
    for (e, mut t, mut vis) in sun {
        let casc = CascadeShadowConfigBuilder {
            num_cascades: cfg.u("casc", 4) as usize,
            maximum_distance: cfg.f("cmax", 150.0),
            ..default()
        }
        .build();
        commands.entity(e).insert(casc);
        if let Ok(mut l) = commands.get_entity(e) {
            l.insert(sun_light(cfg, moon));
        }
        if cfg.u("fog", 0) == 1 && (!night || moon) {
            commands.entity(e).insert(VolumetricLight);
        } else {
            commands.entity(e).remove::<VolumetricLight>();
        }
        *t = Transform::default().looking_to(-sun_dir, Vec3::Y);
        *vis = if night && !moon {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

pub(super) fn apply_lamps_spots(
    cfg: &Cfg,
    commands: &mut Commands,
    lamps: &mut LampQ,
    spots: &mut SpotQ,
    night: bool,
    vspots: usize,
) {
    let lamp_n = cfg.u("lamps", 48) as usize;
    for (l, mut v) in lamps {
        *v = if night && l.0 < lamp_n {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let spot_shadows = cfg.u("spotsh", 0) as usize;
    for (e, h, mut s, mut v) in spots {
        *v = if night && h.0 < cfg.u("spots", 6).max(vspots as u32) as usize {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        s.shadow_maps_enabled = h.0 < spot_shadows.max(vspots);
        if h.0 < vspots {
            commands.entity(e).insert(VolumetricLight);
        } else {
            commands.entity(e).remove::<VolumetricLight>();
        }
    }
}

fn sun_light(cfg: &Cfg, moon: bool) -> DirectionalLight {
    DirectionalLight {
        shadow_maps_enabled: cfg.u("sunshadow", 1) == 1,
        illuminance: if moon {
            cfg.f("lux", 400.0)
        } else {
            cfg.f("lux", 20_000.0)
        },
        ..default()
    }
}
