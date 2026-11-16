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

mod carrier;
mod disc;
pub use carrier::{
    choose_carrier, shadow_carrier_system, ShadowCarrier, CARRIER_HYSTERESIS, MIN_LIGHT_LX,
};
pub use disc::{disc_brightness, setup_moon_disc, MOON_DISC_DAY_BRIGHTNESS, MOON_DISC_SIZE_M};

/// Any sun light (data-driven or fallback).
type SunsQuery<'w, 's> = Query<'w, 's, (), Or<(With<SunLight>, With<FallbackLight>)>>;

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
