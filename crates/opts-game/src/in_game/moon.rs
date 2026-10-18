//! The moon in the game: its light, its disc with the phase, and the choice of the one light
//! that casts shadows (`docs/src/architecture/lighting-system.md` sections 5.3 to 5.5).
//!
//! * [`MoonLight`] is a `DirectionalLight` driven by [`opts_bevy::moon`]: direction from the
//!   moon model, illuminance from the phase, height, sun and weather
//!   ([`opts_bevy::moon::moon_illuminance`]).
//! * The [`MoonDisc`] quad gets a
//!   [`MoonMaterial`] with the phase terminator and sits
//!   where the model puts the moon.
//! * [`shadow_carrier_system`] keeps exactly one directional light (sun or moon) with shadow maps,
//!   switching with a 15 % hysteresis, and hides lights that give no light.

use bevy::prelude::*;
use opts_bevy::moon::{moon_night_factor, MOON_COLOR};
use opts_bevy::moon_material::MoonMaterial;
use opts_bevy::sun_rays::{LightRole, SunRaysSet};
use opts_bevy::SessionEnvironment;

use crate::in_game::sky::{direction, MoonDisc, MoonDiscTexture, SunLight, DOME_RADIUS};
use crate::in_game::types::InGameEntity;
use crate::in_game::world::FallbackLight;

/// Marks the moon's directional light.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MoonLight;

/// Edge of the moon disc quad in metres (7.7° at the dome radius: artistic, the real moon is 0.5°).
pub const MOON_DISC_SIZE_M: f32 = 120.0;
/// Brightness of the disc in full daylight (it is additive, so by day it stays a faint pale disc).
pub const MOON_DISC_DAY_BRIGHTNESS: f32 = 0.35;
/// Relative margin by which the other light must beat the carrier to take over the shadows.
pub const CARRIER_HYSTERESIS: f32 = 0.15;
/// Illuminance (lx) below which a directional light is hidden and cannot carry shadows.
pub const MIN_LIGHT_LX: f32 = 1.0;

/// The light that currently carries the shadows (`None` when neither gives light).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShadowCarrier(pub Option<LightRole>);

/// The carrier after one step. `sun` and `moon` are the scores (illuminance, 0 when the light is
/// below the horizon). The current carrier is kept until the other light beats it by
/// [`CARRIER_HYSTERESIS`] (or the current one falls below half of [`MIN_LIGHT_LX`]); with no
/// carrier, or on a tie, the larger score wins and the sun wins ties.
#[must_use]
pub fn choose_carrier(current: Option<LightRole>, sun: f32, moon: f32) -> Option<LightRole> {
    let score = |r: LightRole| match r {
        LightRole::Sun => sun,
        LightRole::Moon => moon,
    };
    let other = |r: LightRole| match r {
        LightRole::Sun => LightRole::Moon,
        LightRole::Moon => LightRole::Sun,
    };
    match current {
        Some(cur) if score(cur) >= MIN_LIGHT_LX * 0.5 => {
            let o = other(cur);
            if score(o) >= MIN_LIGHT_LX && score(o) > score(cur) * (1.0 + CARRIER_HYSTERESIS) {
                Some(o)
            } else {
                Some(cur)
            }
        }
        _ => {
            let best = if moon > sun {
                LightRole::Moon
            } else {
                LightRole::Sun
            };
            (score(best) >= MIN_LIGHT_LX).then_some(best)
        }
    }
}

/// Spawns the moon light once a sun light exists (the data-driven one or the fallback).
#[allow(clippy::type_complexity)]
pub fn spawn_moon_light(
    mut commands: Commands,
    suns: Query<(), Or<(With<SunLight>, With<FallbackLight>)>>,
    moons: Query<(), With<MoonLight>>,
) {
    if suns.is_empty() || !moons.is_empty() {
        return;
    }
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(MOON_COLOR[0], MOON_COLOR[1], MOON_COLOR[2]),
            illuminance: 0.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::default(),
        Visibility::Hidden,
        MoonLight,
        LightRole::Moon,
        InGameEntity,
        Name::new("Moon_Light"),
    ));
}

/// Gives the moon disc its mesh and phase material (needs the material plugin; without it the
/// disc stays invisible, as in headless tests).
#[allow(clippy::type_complexity)]
pub fn setup_moon_disc(
    mut commands: Commands,
    discs: Query<(Entity, &MoonDiscTexture), (With<MoonDisc>, Without<Mesh3d>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Option<ResMut<Assets<MoonMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    for (entity, texture) in &discs {
        commands.entity(entity).insert((
            Mesh3d(meshes.add(Rectangle::new(MOON_DISC_SIZE_M, MOON_DISC_SIZE_M))),
            MeshMaterial3d(materials.add(MoonMaterial::new(texture.0.clone(), 0.0, 0.0, 1.0))),
            Transform::from_scale(Vec3::ZERO),
        ));
    }
}

/// Brightness of the moon disc: faint by day, full at night, dimmed by cloud.
#[must_use]
pub fn disc_brightness(sun_elevation_deg: f32, weather_moon: f32) -> f32 {
    let night = moon_night_factor(sun_elevation_deg);
    (MOON_DISC_DAY_BRIGHTNESS + (1.0 - MOON_DISC_DAY_BRIGHTNESS) * night) * weather_moon
}

/// Moves the moon light and the disc to the model's position and sets the light and the disc's
/// phase from the session environment.
#[allow(clippy::type_complexity)]
pub fn update_moon(
    env: Res<SessionEnvironment>,
    cameras: Query<&GlobalTransform, (With<opts_bevy::MainCamera>, Without<MoonDisc>)>,
    mut lights: Query<
        (&mut DirectionalLight, &mut Transform),
        (With<MoonLight>, Without<MoonDisc>),
    >,
    mut discs: Query<(&mut Transform, &MeshMaterial3d<MoonMaterial>), With<MoonDisc>>,
    mut materials: Option<ResMut<Assets<MoonMaterial>>>,
) {
    let moon = env.moon();
    let att = env.light_attenuation();
    let dir = direction(moon.elevation_deg, moon.azimuth_deg);
    let lux = opts_bevy::moon::moon_illuminance(moon, env.sun_alt_deg(), att.moon);
    for (mut light, mut t) in &mut lights {
        // A moon below the horizon gives no light (and no shadow score).
        let lux = if moon.elevation_deg > 0.0 { lux } else { 0.0 };
        if (light.illuminance - lux).abs() > 1e-3 {
            light.illuminance = lux;
        }
        let want = Transform::IDENTITY.looking_to(-dir, Vec3::Y);
        if t.rotation != want.rotation {
            t.rotation = want.rotation;
        }
    }
    let Some(cam) = cameras.iter().next().map(GlobalTransform::translation) else {
        return;
    };
    let brightness = disc_brightness(env.sun_alt_deg(), att.moon);
    for (mut t, material) in &mut discs {
        let mut place =
            Transform::from_translation(cam + dir * (DOME_RADIUS - 10.0)).looking_to(dir, Vec3::Y);
        if dir.y <= -0.05 {
            place.scale = Vec3::ZERO;
        }
        *t = place;
        let Some(materials) = materials.as_deref_mut() else {
            continue;
        };
        let want =
            MoonMaterial::new(None, moon.phase_angle_deg, moon.bright_limb_deg, brightness).params;
        let stale = materials
            .get(&material.0)
            .is_some_and(|m| (m.params - want).abs().max_element() > 1e-4);
        if stale {
            if let Some(mut m) = materials.get_mut(&material.0) {
                m.params = want;
            }
        }
    }
}

/// Keeps exactly one directional light with shadow maps ([`choose_carrier`]) and hides the
/// lights that give no light (a hidden light is not extracted for shadow work).
pub fn shadow_carrier_system(
    env: Res<SessionEnvironment>,
    mut carrier: ResMut<ShadowCarrier>,
    mut lights: Query<(&LightRole, &mut DirectionalLight, &mut Visibility)>,
) {
    let score = |role: LightRole, light: &DirectionalLight| {
        let up = match role {
            LightRole::Sun => env.sun_alt_deg() > 0.0,
            LightRole::Moon => env.moon().elevation_deg > 0.0,
        };
        if up {
            light.illuminance
        } else {
            0.0
        }
    };
    let (mut sun, mut moon) = (0.0_f32, 0.0_f32);
    for (role, light, _) in &lights {
        match role {
            LightRole::Sun => sun = sun.max(score(*role, light)),
            LightRole::Moon => moon = moon.max(score(*role, light)),
        }
    }
    let next = choose_carrier(carrier.0, sun, moon);
    if carrier.0 != next {
        carrier.0 = next;
    }
    for (role, mut light, mut visibility) in &mut lights {
        let shadows = next == Some(*role);
        if light.shadow_maps_enabled != shadows {
            light.shadow_maps_enabled = shadows;
        }
        let want = if light.illuminance < MIN_LIGHT_LX {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *visibility != want {
            *visibility = want;
        }
    }
}

/// Registers the moon and the shadow carrier for a drive.
#[derive(Default)]
pub struct MoonPlugin;

impl Plugin for MoonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShadowCarrier>().add_systems(
            Update,
            (
                (spawn_moon_light, setup_moon_disc),
                update_moon
                    .after(crate::in_game::sky::update_sky)
                    .after(crate::in_game::world::update_fallback_light),
                shadow_carrier_system.before(SunRaysSet),
            )
                .chain()
                .after(opts_bevy::EnvironmentSet)
                .run_if(in_state(crate::AppState::InGame)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_bevy::{CalendarDate, EnvironmentChange, EnvironmentPlugin};

    #[test]
    fn the_carrier_keeps_hysteresis_and_prefers_the_sun_on_ties() {
        assert_eq!(choose_carrier(None, 100.0, 100.0), Some(LightRole::Sun));
        assert_eq!(choose_carrier(None, 0.0, 0.0), None);
        assert_eq!(choose_carrier(None, 0.5, 0.2), None);
        assert_eq!(choose_carrier(None, 0.0, 5.0), Some(LightRole::Moon));
        // 14 % better is not enough, 16 % is.
        assert_eq!(
            choose_carrier(Some(LightRole::Sun), 100.0, 114.0),
            Some(LightRole::Sun)
        );
        assert_eq!(
            choose_carrier(Some(LightRole::Sun), 100.0, 116.0),
            Some(LightRole::Moon)
        );
        // A carrier that fell dark hands over to any light.
        assert_eq!(
            choose_carrier(Some(LightRole::Sun), 0.0, 5.0),
            Some(LightRole::Moon)
        );
        assert_eq!(choose_carrier(Some(LightRole::Sun), 0.0, 0.0), None);
    }

    #[test]
    fn the_sun_fading_through_the_hand_over_flips_once() {
        // The sun's illuminance falls from 800 to 0 against a steady 900 lx moon: one switch.
        let mut cur = Some(LightRole::Sun);
        let mut switches = 0;
        for i in 0..=100 {
            let sun = 1500.0 * (1.0 - i as f32 / 100.0);
            let moon = 900.0 + if i % 2 == 0 { 40.0 } else { -40.0 };
            let next = choose_carrier(cur, sun, moon);
            if next != cur {
                switches += 1;
            }
            cur = next;
        }
        assert_eq!(switches, 1);
        assert_eq!(cur, Some(LightRole::Moon));
    }

    #[test]
    fn the_disc_is_faint_by_day_and_dimmed_by_cloud() {
        assert_eq!(disc_brightness(-30.0, 1.0), 1.0);
        assert!((disc_brightness(40.0, 1.0) - MOON_DISC_DAY_BRIGHTNESS).abs() < 1e-6);
        assert!(disc_brightness(-30.0, 0.35) < 0.4);
    }

    fn fake_sun(
        env: Res<SessionEnvironment>,
        mut sun: Query<&mut DirectionalLight, With<SunLight>>,
    ) {
        for mut l in &mut sun {
            l.illuminance = if env.sun_alt_deg() > 0.0 {
                20_000.0
            } else {
                0.0
            };
        }
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(EnvironmentPlugin)
            .init_resource::<ShadowCarrier>()
            .add_systems(
                Update,
                (
                    spawn_moon_light,
                    fake_sun,
                    update_moon,
                    shadow_carrier_system,
                )
                    .chain()
                    .after(opts_bevy::EnvironmentSet),
            );
        app.world_mut().spawn((
            DirectionalLight {
                shadow_maps_enabled: true,
                ..default()
            },
            SunLight,
            LightRole::Sun,
        ));
        app
    }

    fn set_time(app: &mut App, date: CalendarDate, hour: f32) {
        app.world_mut().write_message(EnvironmentChange::reset(
            date,
            f64::from(hour) * 3600.0,
            "Clear Summer Day",
        ));
        app.update();
    }

    /// (shadow-casting lights, moon illuminance, moon hidden)
    fn state(app: &mut App) -> (usize, f32, bool) {
        let w = app.world_mut();
        let shadows = w
            .query::<&DirectionalLight>()
            .iter(w)
            .filter(|l| l.shadow_maps_enabled)
            .count();
        let (l, v) = w
            .query_filtered::<(&DirectionalLight, &Visibility), With<MoonLight>>()
            .single(w)
            .expect("moon light");
        (shadows, l.illuminance, *v == Visibility::Hidden)
    }

    #[test]
    fn one_light_carries_the_shadows_through_a_full_moon_day() {
        let mut app = app();
        // 2026-10-26: full moon (04:12 UTC).
        let date = CalendarDate::new(2026, 10, 26);
        let mut carriers = Vec::new();
        let mut moon_lit_hours = 0;
        for step in 0..96 {
            let hour = step as f32 * 0.25;
            set_time(&mut app, date, hour);
            let (shadows, lux, hidden) = state(&mut app);
            let env = app.world().resource::<SessionEnvironment>();
            let (sun_up, moon_up) = (env.sun_alt_deg() > 0.0, env.moon().elevation_deg > 0.0);
            assert!(shadows <= 1, "{hour}: {shadows} shadow lights");
            let carrier = app.world().resource::<ShadowCarrier>().0;
            if sun_up {
                if env.sun_alt_deg() >= 6.0 {
                    assert_eq!(lux, 0.0, "{hour}: no moonlight by day");
                    assert!(hidden, "{hour}");
                }
                assert_eq!(carrier, Some(LightRole::Sun), "{hour}");
                assert_eq!(shadows, 1);
            } else if moon_up && lux >= MIN_LIGHT_LX {
                moon_lit_hours += 1;
                assert_eq!(carrier, Some(LightRole::Moon), "{hour}: lux {lux}");
                assert_eq!(shadows, 1);
                assert!(!hidden);
            } else {
                assert_eq!(carrier, None, "{hour}");
                assert_eq!(shadows, 0, "{hour}");
            }
            if carriers.last() != Some(&carrier) {
                carriers.push(carrier);
            }
        }
        assert!(moon_lit_hours >= 4 * 6, "the full moon lights the night");
        // Sun, (nothing or moon) ..., sun: no flicker between.
        assert!(carriers.len() <= 5, "{carriers:?}");
    }

    #[test]
    fn a_new_moon_night_has_no_moonlight_and_no_shadows() {
        let mut app = app();
        // 2026-11-09: new moon (about 08:00 UTC).
        let date = CalendarDate::new(2026, 11, 9);
        for hour in [0.0, 2.0, 22.0, 23.5] {
            set_time(&mut app, date, hour);
            let (shadows, lux, hidden) = state(&mut app);
            assert_eq!(lux, 0.0, "{hour}");
            assert!(hidden);
            assert_eq!(shadows, 0, "{hour}");
        }
        set_time(&mut app, date, 12.0);
        assert_eq!(state(&mut app).0, 1, "the sun carries by day");
    }

    #[test]
    fn the_moon_light_points_from_the_model_direction() {
        let mut app = app();
        set_time(&mut app, CalendarDate::new(2026, 10, 26), 0.0);
        let env = app.world().resource::<SessionEnvironment>().clone();
        let moon = env.moon();
        assert!(moon.elevation_deg > 0.0, "{moon:?}");
        let t = app
            .world_mut()
            .query_filtered::<&Transform, With<MoonLight>>()
            .single(app.world())
            .unwrap();
        let towards_moon = -*t.forward();
        let want = direction(moon.elevation_deg, moon.azimuth_deg);
        assert!(towards_moon.dot(want) > 0.999);
    }
}
