//! Applying a plan item's keys to the scene.

mod scene;

use bevy::ecs::system::SystemParam;

use super::cone_material::ConeMaterial;
use super::config::Cfg;
use super::procedural::sky_cube;
use super::*;

type StaleQ<'w, 's> = Query<'w, 's, Entity, Or<(With<Cone>, With<Fog>, With<Mirror>)>>;
type SunQ<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static mut Transform, &'static mut Visibility),
    (With<Sun>, Without<Cam>),
>;
type LampQ<'w, 's> =
    Query<'w, 's, (&'static NightLamp, &'static mut Visibility), (Without<Sun>, Without<HeadSpot>)>;
type SpotQ<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static HeadSpot,
        &'static mut SpotLight,
        &'static mut Visibility,
    ),
    (Without<Sun>, Without<NightLamp>),
>;

/// Everything `apply` touches besides `Commands` and the plan item.
#[derive(SystemParam)]
pub(super) struct ApplyParams<'w, 's> {
    pub(super) assets: Res<'w, Assets3>,
    cones: ResMut<'w, Assets<ConeMaterial>>,
    shadow_map: ResMut<'w, DirectionalLightShadowMap>,
    cam: Query<'w, 's, Entity, With<Cam>>,
    images: ResMut<'w, Assets<Image>>,
    every: ResMut<'w, MirrorEvery>,
    stale_q: StaleQ<'w, 's>,
    sun: SunQ<'w, 's>,
    lamps: LampQ<'w, 's>,
    spots: SpotQ<'w, 's>,
    ambient: ResMut<'w, GlobalAmbientLight>,
}

pub(super) fn apply(cfg: &Cfg, commands: &mut Commands, p: &mut ApplyParams) {
    let Some(cam) = p.cam.single().ok() else {
        return;
    };
    let night = cfg.s("scene", "sun") == "night";
    p.shadow_map.size = cfg.u("shadow", 2048) as usize;
    commands.insert_resource(ClearColor(if night {
        Color::srgb(0.01, 0.015, 0.03)
    } else {
        Color::srgb(0.52, 0.70, 0.95)
    }));

    let toward = cfg.s("look", "toward") == "toward";
    let moon = cfg.u("moon", 0) == 1;
    apply_sun(cfg, commands, &mut p.sun, night, moon, toward);
    p.ambient.brightness = if night { 20.0 } else { 700.0 };
    let vspots = cfg.u("vspots", 0) as usize;
    apply_lamps_spots(cfg, commands, &mut p.lamps, &mut p.spots, night, vspots);

    // Camera.
    // "toward" looks down -Z (toward the sun); "away" turns around and looks down +Z, the camera
    // sits at the far end of the street so the same canyon fills the view.
    let pos = if toward {
        Vec3::new(0.0, 1.8, 6.0)
    } else {
        Vec3::new(0.0, 1.8, -140.0)
    };
    let look_dir = if toward { Vec3::NEG_Z } else { Vec3::Z };
    apply_camera(cfg, commands, cam, &mut p.images, pos, look_dir);
    // Mirror cameras (render to a small 8-bit image; every `mevery`-th frame).
    for e in &p.stale_q {
        commands.entity(e).despawn();
    }
    p.every.0 = cfg.u("mevery", 1);
    scene::spawn_mirrors(cfg, commands, &mut p.images, pos, look_dir);
    scene::spawn_ballast(cfg, commands, &p.assets, &mut p.cones, pos, look_dir);
    apply_prepass_fog(cfg, commands, cam, vspots);
    scene::spawn_fog_volume(cfg, commands, vspots, pos);
    scene::spawn_cones(cfg, commands, &p.assets, &mut p.cones, pos, look_dir);
}

fn apply_sun(
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

fn apply_lamps_spots(
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

fn apply_camera(
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

fn apply_prepass_fog(cfg: &Cfg, commands: &mut Commands, cam: Entity, vspots: usize) {
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

pub(super) fn mirror_toggle(
    bench: Res<Bench>,
    every: Res<MirrorEvery>,
    mut q: Query<&mut Camera, With<Mirror>>,
) {
    let on = every.0 <= 1 || bench.frame.is_multiple_of(every.0);
    for mut c in &mut q {
        if c.is_active != on {
            c.is_active = on;
        }
    }
}
