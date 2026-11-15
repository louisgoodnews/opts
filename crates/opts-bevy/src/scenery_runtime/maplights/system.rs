//! The map light pool system: camera-driven lamp choice, interior scaling and per-slot update.

use bevy::prelude::*;

use super::component::MapLight;
use super::photometry::choose_nearest;
use super::pool::{assign, ensure_pool_capacity, MapLightPool};
use super::settings::MapLightSettings;
use super::slot_light::update_slot_light;
use crate::scenery_runtime::clock::SceneryClock;

/// System parameters for the map light pooling system.
#[derive(bevy::ecs::system::SystemParam)]
pub struct MapLightPoolParams<'w, 's> {
    pub time: Res<'w, Time<Real>>,
    pub settings: Res<'w, MapLightSettings>,
    pub clock: Res<'w, SceneryClock>,
    pub pool: ResMut<'w, MapLightPool>,
    pub cameras:
        Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<crate::MainCamera>>,
    pub lamps: Query<'w, 's, (Entity, &'static MapLight, &'static GlobalTransform)>,
    pub lights: Query<
        'w,
        's,
        (
            &'static mut PointLight,
            &'static mut Transform,
            &'static mut Visibility,
        ),
        Without<MapLight>,
    >,
    pub cabin: Option<Res<'w, crate::light_rooms::CabinView>>,
    pub vehicle: Query<
        'w,
        's,
        (
            &'static GlobalTransform,
            &'static crate::light_rooms::VehicleFootprint,
        ),
        With<crate::PlayerVehicle>,
    >,
}

/// The lamps that get a pooled light: the nearest to the camera within the budget and radius,
/// with hysteresis, leaving out the lamps above the player's vehicle in the interior view.
fn choose_lamps(p: &MapLightPoolParams, cam: Vec3, interior: bool) -> Vec<Entity> {
    let hide_over = interior && p.settings.exclude_over_vehicle;
    let candidates: Vec<(Entity, f32)> = p
        .lamps
        .iter()
        .filter(|(_, _, t)| {
            !hide_over
                || !p.vehicle.iter().any(|(g, fp)| {
                    crate::light_rooms::over_vehicle(
                        t.translation(),
                        g,
                        fp,
                        crate::light_rooms::FOOTPRINT_MARGIN_M,
                    )
                })
        })
        .map(|(e, _, t)| (e, t.translation().distance(cam)))
        .collect();
    let current: Vec<Entity> = p
        .pool
        .slots
        .iter()
        .filter(|s| s.active)
        .filter_map(|s| s.target)
        .collect();
    choose_nearest(
        &candidates,
        &current,
        p.settings.pool_size,
        p.settings.radius_m,
        p.settings.hysteresis_m,
    )
}

/// Hands the pool's lights to the nearest lamps, fades them and switches them by night.
pub fn map_light_pool_system(
    mut commands: Commands,
    mut p: MapLightPoolParams,
    mut view_scale: Local<Option<f32>>,
) {
    if !p.settings.enabled {
        return;
    }
    if ensure_pool_capacity(&mut commands, &mut p.pool, p.settings.pool_size) {
        return;
    }
    let dt = p.time.delta_secs();
    p.pool.since_assign_s += dt;
    let interior = p.cabin.as_deref().is_some_and(|c| c.interior_view);
    let scale_target = if interior {
        p.settings.interior_view_scale
    } else {
        1.0
    };
    let scale = view_scale.get_or_insert(scale_target);
    let scale_step = if p.settings.fade_s > 0.0 {
        dt / p.settings.fade_s
    } else {
        1.0
    };
    *scale += (scale_target - *scale).clamp(-scale_step, scale_step);
    let scale = *scale;
    let camera = p
        .cameras
        .iter()
        .find(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation());
    if p.pool.since_assign_s >= p.settings.interval_s {
        p.pool.since_assign_s = 0.0;
        let chosen = match camera.filter(|_| p.clock.night) {
            Some(cam) => choose_lamps(&p, cam, interior),
            None => Vec::new(),
        };
        assign(&mut p.pool.slots, &chosen);
    }
    let step = if p.settings.fade_s > 0.0 {
        dt / p.settings.fade_s
    } else {
        1.0
    };
    for slot in &mut p.pool.slots {
        let lamp = slot.target.and_then(|t| p.lamps.get(t).ok());
        update_slot_light(
            slot,
            lamp,
            &mut p.lights,
            step,
            p.clock.night,
            scale,
            &p.settings,
        );
    }
}
