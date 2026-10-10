//! Atmospheric lighting for the in-game scene. World geometry is loaded from OMSI content
//! by [`crate::in_game::omsi_world`].

use bevy::prelude::*;

use crate::in_game::types::InGameEntity;
use crate::new_game_screen::types::NewGameSetup;

/// Spawns atmospheric lighting (sun / moon) and ambient illumination based on departure time & weather.
pub fn spawn_atmosphere_and_lighting(commands: &mut Commands, setup: &NewGameSetup) {
    let hour = setup.hour as f32 + (setup.minute as f32 / 60.0);
    let is_night = !(6.0..20.0).contains(&hour);
    let is_golden_hour = (6.0..8.0).contains(&hour) || (18.0..20.0).contains(&hour);

    let weather = setup.weather_preset.to_lowercase();
    let is_rain_or_fog =
        weather.contains("rain") || weather.contains("fog") || weather.contains("snow");
    let is_overcast = weather.contains("overcast");

    // 1. Calculate Sun Position Angle
    // In Bevy: -Z is forward, +Y is up, +X is right.
    let (sun_rotation, sun_color, illuminance) = if is_night {
        // Night: Cool moonlight from high angle
        (
            Quat::from_euler(EulerRot::XYZ, -0.9, 0.4, 0.0),
            Color::srgb(0.45, 0.55, 0.85),
            900.0,
        )
    } else if is_golden_hour {
        // Sunrise / Sunset: Low warm amber sun
        let yaw = if hour < 12.0 { 0.3 } else { 2.8 };
        (
            Quat::from_euler(EulerRot::XYZ, -0.3, yaw, 0.0),
            Color::srgb(1.0, 0.75, 0.45),
            12_000.0,
        )
    } else if is_rain_or_fog || is_overcast {
        // Overcast / Rain: Diffuse silvery sky
        (
            Quat::from_euler(EulerRot::XYZ, -1.1, 0.6, 0.0),
            Color::srgb(0.8, 0.85, 0.9),
            7_000.0,
        )
    } else {
        // Clear Day: Bright golden-white solar illumination
        let progress_through_day = ((hour - 6.0) / 14.0).clamp(0.0, 1.0);
        let pitch = -0.4 - 0.7 * (1.0 - (progress_through_day - 0.5).abs() * 2.0);
        let yaw = 0.2 + progress_through_day * 1.5;
        (
            Quat::from_euler(EulerRot::XYZ, pitch, yaw, 0.0),
            Color::srgb(1.0, 0.98, 0.92),
            18_000.0,
        )
    };

    commands.spawn((
        (
            DirectionalLight {
                color: sun_color,
                illuminance,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_rotation(sun_rotation),
        ),
        InGameEntity,
        Name::new("Atmosphere_Sun_Light"),
    ));

    // 2. Ambient Lighting
    let (ambient_color, ambient_brightness) = if is_night {
        (Color::srgb(0.12, 0.15, 0.28), 120.0)
    } else if is_rain_or_fog || is_overcast {
        (Color::srgb(0.65, 0.7, 0.75), 380.0)
    } else {
        (Color::srgb(0.88, 0.92, 1.0), 480.0)
    };

    commands.insert_resource(GlobalAmbientLight {
        color: ambient_color,
        brightness: ambient_brightness,
        ..default()
    });
}
