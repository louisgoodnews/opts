//! Applying a plan item's keys to the scene.

mod camera;
mod cones;
mod lights;
mod scene;

use bevy::ecs::system::SystemParam;

use super::cone_material::ConeMaterial;
use super::config::Cfg;

use super::*;
use camera::{apply_camera, apply_prepass_fog};
use lights::{apply_lamps_spots, apply_sun};

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
    cones::spawn_cones(cfg, commands, &p.assets, &mut p.cones, pos, look_dir);
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
