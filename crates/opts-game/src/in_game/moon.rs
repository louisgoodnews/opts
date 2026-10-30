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

/// Any sun light (data-driven or fallback).
type SunsQuery<'w, 's> = Query<'w, 's, (), Or<(With<SunLight>, With<FallbackLight>)>>;
/// Moon discs still without a mesh.
type NewDiscsQuery<'w, 's> =
    Query<'w, 's, (Entity, &'static MoonDiscTexture), (With<MoonDisc>, Without<Mesh3d>)>;
/// The main camera's transform.
type CameraQuery<'w, 's> =
    Query<'w, 's, &'static GlobalTransform, (With<opts_bevy::MainCamera>, Without<MoonDisc>)>;
/// The moon light's light and transform.
type MoonLightQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut DirectionalLight, &'static mut Transform),
    (With<MoonLight>, Without<MoonDisc>),
>;
/// The moon disc's transform and material.
type MoonDiscQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        &'static MeshMaterial3d<MoonMaterial>,
    ),
    With<MoonDisc>,
>;

/// Spawns the moon light once a sun light exists (the data-driven one or the fallback).
pub fn spawn_moon_light(
    mut commands: Commands,
    suns: SunsQuery,
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
pub fn setup_moon_disc(
    mut commands: Commands,
    discs: NewDiscsQuery,
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
pub fn update_moon(
    env: Res<SessionEnvironment>,
    cameras: CameraQuery,
    mut lights: MoonLightQuery,
    mut discs: MoonDiscQuery,
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
mod tests;
