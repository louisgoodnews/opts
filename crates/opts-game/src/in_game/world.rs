//! Fallback lighting for the in-game scene when the data-driven sky (`sky.rs`) is unavailable
//! (no `envir.cfg` or sky textures). World geometry is loaded from OMSI content by
//! [`crate::in_game::omsi_world`].
//!
//! The light follows the [`SessionEnvironment`] like the sky does: [`update_fallback_light`]
//! sets it whenever the environment changes. At night it gives no direct light; the moon light
//! (`in_game::moon`) is the one night light in both the data-driven and the fallback path.

use bevy::prelude::*;
use opts_bevy::{SessionEnvironment, WeatherKind};

use crate::in_game::types::InGameEntity;

/// Marks the fallback sun light.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FallbackLight;

/// The fallback sun and ambient light for an hour of the day and a weather.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FallbackLighting {
    /// Orientation of the directional light.
    pub rotation: Quat,
    /// Colour of the directional light.
    pub color: Color,
    /// Illuminance of the directional light.
    pub illuminance: f32,
    /// Ambient colour.
    pub ambient_color: Color,
    /// Ambient brightness.
    pub ambient_brightness: f32,
}

/// Picks the fallback light for the sun's position: a low warm sun in the golden hours, a
/// diffuse sky under clouds, a bright sun otherwise, and **no direct light at night**: the moon
/// ([`crate::in_game::moon`]) is the one night light. The direction is the sun's.
#[must_use]
pub fn fallback_lighting(
    sun_alt_deg: f32,
    sun_az_deg: f32,
    weather: WeatherKind,
) -> FallbackLighting {
    let is_night = sun_alt_deg < 0.0;
    let is_golden_hour = sun_alt_deg < 12.0;
    let covered = weather.overcast();
    // The sun fades out through the twilight instead of switching off.
    let fade = ((sun_alt_deg + 4.0) / 7.0).clamp(0.0, 1.0);
    let fade = fade * fade * (3.0 - 2.0 * fade);

    // In Bevy: -Z is forward, +Y is up, +X is right.
    let dir = crate::in_game::sky::direction(sun_alt_deg, sun_az_deg);
    let rotation = Transform::IDENTITY.looking_to(-dir, Vec3::Y).rotation;
    let (color, illuminance) = if is_golden_hour {
        // Sunrise / sunset: a low warm sun.
        (Color::srgb(1.0, 0.75, 0.45), 12_000.0)
    } else if covered {
        // Overcast, rain, snow, fog: a diffuse silvery sky.
        (Color::srgb(0.8, 0.85, 0.9), 7_000.0)
    } else {
        // Clear day: bright golden-white sun.
        (Color::srgb(1.0, 0.98, 0.92), 18_000.0)
    };
    let (ambient_color, ambient_brightness) = if is_night {
        (Color::srgb(0.12, 0.15, 0.28), 120.0)
    } else if covered {
        (Color::srgb(0.65, 0.7, 0.75), 380.0)
    } else {
        (Color::srgb(0.88, 0.92, 1.0), 480.0)
    };
    FallbackLighting {
        rotation,
        color,
        illuminance: illuminance * fade,
        ambient_color,
        ambient_brightness,
    }
}

/// Spawns the fallback sun light; [`update_fallback_light`] sets it from the environment.
pub fn spawn_atmosphere_and_lighting(commands: &mut Commands) {
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::default(),
        FallbackLight,
        opts_bevy::sun_rays::LightRole::Sun,
        InGameEntity,
        Name::new("Atmosphere_Sun_Light"),
    ));
}

/// Sets the fallback light and the ambient light from the session environment (when it
/// changes, and once for a new light).
pub fn update_fallback_light(
    env: Res<SessionEnvironment>,
    mut lights: Query<(&mut DirectionalLight, &mut Transform, Ref<FallbackLight>)>,
    mut commands: Commands,
    mut last: Local<Option<FallbackLighting>>,
) {
    if lights.is_empty() {
        *last = None;
        return;
    }
    let added = lights.iter().any(|(_, _, f)| f.is_added());
    let want = fallback_lighting(env.sun_alt_deg(), env.sun_az_deg(), env.weather_kind());
    if !added && *last == Some(want) {
        return;
    }
    // The sun moves, so the light changes every tick; the ambient light only changes rarely.
    let ambient_same = !added
        && last.is_some_and(|l| {
            l.ambient_color == want.ambient_color && l.ambient_brightness == want.ambient_brightness
        });
    *last = Some(want);
    for (mut light, mut transform, _) in &mut lights {
        light.color = want.color;
        light.illuminance = want.illuminance;
        transform.rotation = want.rotation;
    }
    if !ambient_same {
        commands.insert_resource(GlobalAmbientLight {
            color: want.ambient_color,
            brightness: want.ambient_brightness,
            ..default()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_bevy::{CalendarDate, EnvironmentChange, EnvironmentPlugin};

    #[test]
    fn the_light_follows_the_sun_and_weather() {
        let night = fallback_lighting(-30.0, 0.0, WeatherKind::Clear);
        assert_eq!(night.illuminance, 0.0, "the moon is the night light");
        assert_eq!(night.ambient_brightness, 120.0);
        let noon = fallback_lighting(60.0, 180.0, WeatherKind::Clear);
        assert_eq!(noon.illuminance, 18_000.0);
        let grey = fallback_lighting(60.0, 180.0, WeatherKind::Rain);
        assert_eq!(grey.illuminance, 7_000.0);
        assert_eq!(grey.ambient_brightness, 380.0);
        // Low sun: warm and dimmer through the twilight.
        let low = fallback_lighting(5.0, 90.0, WeatherKind::Clear);
        assert!(low.illuminance > 0.0 && low.illuminance <= 12_000.0);
        assert!(fallback_lighting(1.0, 90.0, WeatherKind::Clear).illuminance < low.illuminance);
    }

    #[test]
    fn the_system_updates_the_light_when_the_time_changes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(EnvironmentPlugin)
            .add_systems(
                Update,
                update_fallback_light.after(opts_bevy::EnvironmentSet),
            );
        app.world_mut().write_message(EnvironmentChange::reset(
            CalendarDate::new(1994, 6, 1),
            12.0 * 3600.0,
            "Clear Summer Day",
        ));
        spawn_atmosphere_and_lighting(&mut app.world_mut().commands());
        app.update();
        let illum = |app: &mut App| {
            app.world_mut()
                .query_filtered::<&DirectionalLight, With<FallbackLight>>()
                .single(app.world())
                .expect("light")
                .illuminance
        };
        assert_eq!(illum(&mut app), 18_000.0);
        app.world_mut().write_message(EnvironmentChange {
            seconds_of_day: Some(23.0 * 3600.0),
            ..default()
        });
        app.update();
        assert_eq!(illum(&mut app), 0.0);
        assert_eq!(
            app.world().resource::<GlobalAmbientLight>().brightness,
            120.0
        );
    }
}
