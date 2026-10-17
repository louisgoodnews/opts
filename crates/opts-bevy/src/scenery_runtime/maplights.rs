//! `[maplight]` ground lights of scenery objects (street lamps: 1,771 on Berlin-Spandau).
//!
//! Every `[maplight]` becomes a [`MapLight`] marker entity under its object; no Bevy light is
//! created per lamp. A fixed pool of shadowless point lights ([`MapLightPool`]) is handed every
//! [`MapLightSettings::interval_s`] to the nearest lamps within [`MapLightSettings::radius_m`] of
//! the camera (with hysteresis, so a lamp at the edge does not flicker in and out), switched by
//! the night switch (`NightlightA`, [`SceneryClock::night`]) and faded in and out.
//!
//! Photometry: a `[maplight]` gives the radius `r` at which the light is at full intensity, then
//! falls off with the square of the distance (the comment in the stock lamp objects). With a full
//! illuminance `E_full` at `r`, the point light's luminous power is `4π·r²·E_full` lumens; its range
//! is `8·r`. `E_full` is a tuning value ([`MapLightSettings::full_lux`]).

use bevy::prelude::*;
use opts_core::formats::scenery::SceneryObject;

use super::clock::SceneryClock;
use crate::coords::omsi_to_bevy_pos;

/// One `[maplight]` of a placed object (its entity sits at the light's position).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct MapLight {
    /// Colour (0–1 per channel, as written).
    pub color: Vec3,
    /// Radius at full intensity (m).
    pub radius_m: f32,
}

/// Illuminance of a `[maplight]` at its full-intensity radius (lx). OMSI's own description:
/// full intensity within the radius, then falling off with the square of the distance. "Full"
/// is the scene's full lighting level, so it is set to half the midday sun of the session sky
/// (20,000 lx): a lamp 7 m above the road then lights it with about 1,300 lx, which reads as a
/// lit patch under the game's fixed exposure. The physical value of a street lamp (≈ 15 lx)
/// is invisible at that exposure.
pub const FULL_LUX: f32 = 10_000.0;

/// Tuning of the map light pool.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct MapLightSettings {
    /// Map lights are drawn at all.
    pub enabled: bool,
    /// Point lights in the pool.
    pub pool_size: usize,
    /// Only lamps within this distance of the camera get a light (m).
    pub radius_m: f32,
    /// A lamp that has a light keeps it while it is at most this much farther than the
    /// farthest newly chosen lamp (m).
    pub hysteresis_m: f32,
    /// Seconds between two assignments.
    pub interval_s: f32,
    /// Illuminance at the full-intensity radius (lx).
    pub full_lux: f32,
    /// Fade time in and out (s).
    pub fade_s: f32,
    /// While the camera is in an interior view of the player vehicle, lamps over the vehicle
    /// (inside its footprint prism, from the roof up) get no light (`lighting-system.md` 4.5).
    pub exclude_over_vehicle: bool,
    /// Factor on the pool's intensity while the camera is in an interior view, so a lamp outside
    /// does not out-shine the cabin light ([`crate::light_rooms::INTERIOR_VIEW_SCALE`]).
    pub interior_view_scale: f32,
}

impl Default for MapLightSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            pool_size: 48,
            radius_m: 200.0,
            hysteresis_m: 15.0,
            interval_s: 0.25,
            full_lux: FULL_LUX,
            fade_s: 0.5,
            exclude_over_vehicle: true,
            interior_view_scale: crate::light_rooms::INTERIOR_VIEW_SCALE,
        }
    }
}

/// One light of the pool.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolSlot {
    /// The pool's point light entity.
    pub light: Entity,
    /// The lamp it shows (`None`: free).
    pub target: Option<Entity>,
    /// The lamp is still chosen (otherwise the slot fades out and becomes free).
    pub active: bool,
    /// Fade level 0–1.
    pub level: f32,
}

/// The pool of point lights shared by all `[maplight]`s.
#[derive(Resource, Debug, Default, Clone)]
pub struct MapLightPool {
    /// The pool's lights (spawned on first use).
    pub slots: Vec<PoolSlot>,
    /// Seconds since the last assignment.
    pub since_assign_s: f32,
}

impl MapLightPool {
    /// Lamps that currently hold a light (fading out ones included).
    #[must_use]
    pub fn assigned(&self) -> Vec<Entity> {
        self.slots.iter().filter_map(|s| s.target).collect()
    }
}

/// Luminous power (lm) of a lamp with full-intensity radius `radius_m` and full illuminance
/// `full_lux`: `4π·r²·E`.
#[must_use]
pub fn map_light_lumens(radius_m: f32, full_lux: f32) -> f32 {
    4.0 * std::f32::consts::PI * radius_m * radius_m * full_lux
}

/// Range of a lamp's point light (m): `8·r`.
#[must_use]
pub fn map_light_range(radius_m: f32) -> f32 {
    8.0 * radius_m
}

/// The lamps that should hold a light: the `n` nearest of `candidates` (lamp, distance) within
/// `radius_m`, where lamps in `current` count `hysteresis_m` nearer than they are. Ties break by
/// entity, so the choice is stable from one assignment to the next.
#[must_use]
pub fn choose_nearest(
    candidates: &[(Entity, f32)],
    current: &[Entity],
    n: usize,
    radius_m: f32,
    hysteresis_m: f32,
) -> Vec<Entity> {
    let mut ranked: Vec<(f32, Entity)> = candidates
        .iter()
        .filter_map(|&(e, d)| {
            let held = current.contains(&e);
            let reach = if held {
                radius_m + hysteresis_m
            } else {
                radius_m
            };
            (d <= reach).then_some((if held { d - hysteresis_m } else { d }, e))
        })
        .collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    ranked.into_iter().take(n).map(|(_, e)| e).collect()
}

/// Spawns a [`MapLight`] child of `root` for every `[maplight]` of `sco`; returns how many.
pub fn spawn_map_lights(commands: &mut Commands, root: Entity, sco: &SceneryObject) -> usize {
    for light in &sco.map_lights {
        let e = commands
            .spawn((
                Transform::from_translation(omsi_to_bevy_pos(light.position)),
                MapLight {
                    color: light.color,
                    radius_m: light.radius.max(0.1),
                },
                Name::new("MapLight"),
            ))
            .id();
        commands.entity(root).add_child(e);
    }
    sco.map_lights.len()
}

/// Hands the pool's lights to the nearest lamps, fades them and switches them by night.
#[allow(clippy::too_many_arguments)]
pub fn map_light_pool_system(
    mut commands: Commands,
    time: Res<Time>,
    settings: Res<MapLightSettings>,
    clock: Res<SceneryClock>,
    mut pool: ResMut<MapLightPool>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::MainCamera>>,
    lamps: Query<(Entity, &MapLight, &GlobalTransform)>,
    mut lights: Query<(&mut PointLight, &mut Transform, &mut Visibility), Without<MapLight>>,
    cabin: Option<Res<crate::light_rooms::CabinView>>,
    vehicle: Query<
        (&GlobalTransform, &crate::light_rooms::VehicleFootprint),
        With<crate::PlayerVehicle>,
    >,
    mut view_scale: Local<Option<f32>>,
) {
    if !settings.enabled {
        return;
    }
    if pool.slots.len() < settings.pool_size {
        for _ in pool.slots.len()..settings.pool_size {
            let light = commands
                .spawn((
                    PointLight {
                        intensity: 0.0,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    Transform::default(),
                    Visibility::Hidden,
                    Name::new("MapLightPool"),
                ))
                .id();
            pool.slots.push(PoolSlot {
                light,
                target: None,
                active: false,
                level: 0.0,
            });
        }
        // Lights spawn this frame; the next one drives them.
        return;
    }
    let dt = time.delta_secs();
    pool.since_assign_s += dt;
    let interior = cabin.as_deref().is_some_and(|c| c.interior_view);
    // The interior-view dimming fades with the lamps instead of jumping.
    let scale_target = if interior {
        settings.interior_view_scale
    } else {
        1.0
    };
    let scale = view_scale.get_or_insert(scale_target);
    let scale_step = if settings.fade_s > 0.0 {
        dt / settings.fade_s
    } else {
        1.0
    };
    *scale += (scale_target - *scale).clamp(-scale_step, scale_step);
    let scale = *scale;
    let camera = cameras
        .iter()
        .find(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation());
    if pool.since_assign_s >= settings.interval_s {
        pool.since_assign_s = 0.0;
        let chosen = match camera.filter(|_| clock.night) {
            Some(cam) => {
                let hide_over = interior && settings.exclude_over_vehicle;
                let candidates: Vec<(Entity, f32)> = lamps
                    .iter()
                    .filter(|(_, _, t)| {
                        !hide_over
                            || !vehicle.iter().any(|(g, fp)| {
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
                let current: Vec<Entity> = pool
                    .slots
                    .iter()
                    .filter(|s| s.active)
                    .filter_map(|s| s.target)
                    .collect();
                choose_nearest(
                    &candidates,
                    &current,
                    settings.pool_size,
                    settings.radius_m,
                    settings.hysteresis_m,
                )
            }
            None => Vec::new(),
        };
        assign(&mut pool.slots, &chosen);
    }
    let step = if settings.fade_s > 0.0 {
        dt / settings.fade_s
    } else {
        1.0
    };
    for slot in &mut pool.slots {
        let lamp = slot.target.and_then(|t| lamps.get(t).ok());
        let want = if slot.active && clock.night && lamp.is_some() {
            1.0
        } else {
            0.0
        };
        slot.level = if want > slot.level {
            (slot.level + step).min(want)
        } else {
            (slot.level - step).max(want)
        };
        if slot.level <= 0.0 && !slot.active {
            slot.target = None;
        }
        let Ok((mut point, mut transform, mut visibility)) = lights.get_mut(slot.light) else {
            continue;
        };
        match lamp.filter(|_| slot.level > 0.0) {
            Some((_, m, t)) => {
                point.color = Color::srgb(m.color.x, m.color.y, m.color.z);
                point.intensity =
                    map_light_lumens(m.radius_m, settings.full_lux) * slot.level * scale;
                point.range = map_light_range(m.radius_m);
                transform.translation = t.translation();
                if *visibility != Visibility::Inherited {
                    *visibility = Visibility::Inherited;
                }
            }
            None => {
                if *visibility != Visibility::Hidden {
                    *visibility = Visibility::Hidden;
                    point.intensity = 0.0;
                }
            }
        }
    }
}

/// Keeps slots whose lamp is still chosen, lets the others fade out and gives free slots to the
/// chosen lamps that have none.
pub fn assign(slots: &mut [PoolSlot], chosen: &[Entity]) {
    for slot in slots.iter_mut() {
        slot.active = slot.target.is_some_and(|t| chosen.contains(&t));
    }
    let held: Vec<Entity> = slots.iter().filter_map(|s| s.target).collect();
    let mut waiting = chosen.iter().filter(|e| !held.contains(e));
    for slot in slots.iter_mut().filter(|s| s.target.is_none()) {
        let Some(&next) = waiting.next() else {
            break;
        };
        *slot = PoolSlot {
            light: slot.light,
            target: Some(next),
            active: true,
            level: 0.0,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(i: u32) -> Entity {
        Entity::from_raw_u32(i).expect("entity")
    }

    #[test]
    fn the_nearest_n_within_the_radius_are_chosen() {
        let cands: Vec<(Entity, f32)> = (1..=10).map(|i| (e(i), i as f32 * 30.0)).collect();
        let chosen = choose_nearest(&cands, &[], 3, 200.0, 15.0);
        assert_eq!(chosen, vec![e(1), e(2), e(3)]);
        // 7 × 30 = 210 m is outside the radius.
        assert_eq!(choose_nearest(&cands, &[], 20, 200.0, 15.0).len(), 6);
    }

    #[test]
    fn a_held_lamp_keeps_its_light_against_a_slightly_nearer_one() {
        let cands = [(e(1), 50.0), (e(2), 60.0), (e(3), 55.0)];
        // e(2) holds a light; e(3) is 5 m nearer but within the hysteresis.
        let chosen = choose_nearest(&cands, &[e(2)], 2, 200.0, 15.0);
        assert_eq!(chosen, vec![e(2), e(1)]);
        // Repeating the choice gives the same set (stable).
        let mut again = choose_nearest(&cands, &chosen, 2, 200.0, 15.0);
        again.sort();
        let mut first = chosen.clone();
        first.sort();
        assert_eq!(again, first);
        // Without the light it would lose.
        assert_eq!(
            choose_nearest(&cands, &[], 2, 200.0, 15.0),
            vec![e(1), e(3)]
        );
    }

    #[test]
    fn slots_keep_their_lamp_and_released_ones_fade_before_reuse() {
        let mut slots: Vec<PoolSlot> = (100..102)
            .map(|i| PoolSlot {
                light: e(i),
                target: None,
                active: false,
                level: 0.0,
            })
            .collect();
        assign(&mut slots, &[e(1), e(2)]);
        assert_eq!(slots[0].target, Some(e(1)));
        assert_eq!(slots[1].target, Some(e(2)));
        slots[0].level = 1.0;
        slots[1].level = 1.0;
        // e(2) leaves, e(3) arrives: e(1) keeps its slot; e(2)'s slot fades out first.
        assign(&mut slots, &[e(1), e(3)]);
        assert_eq!(slots[0].target, Some(e(1)));
        assert!(slots[0].active);
        assert_eq!(slots[1].target, Some(e(2)));
        assert!(!slots[1].active);
        slots[1].target = None; // faded out
        assign(&mut slots, &[e(1), e(3)]);
        assert_eq!(slots[1].target, Some(e(3)));
        assert!(slots[1].active && slots[1].level == 0.0);
    }

    #[test]
    fn photometry_follows_the_full_radius() {
        let lm = map_light_lumens(4.5, 15.0);
        // Illuminance at r: (Φ / 4π) / r² = E_full.
        let e_at_r = lm / (4.0 * std::f32::consts::PI) / (4.5 * 4.5);
        assert!((e_at_r - 15.0).abs() < 1e-3);
        assert!((map_light_range(4.5) - 36.0).abs() < 1e-6);
    }
}
