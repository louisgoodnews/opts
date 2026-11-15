//! The lights of the street: the lamps along the road.

use super::*;

pub(super) fn spawn_lights(commands: &mut Commands) {
    // Street lamps: 48 map point lights (as MapLightPool: r = 4.5 m, 10 klx at r).
    let lumens = 4.0 * std::f32::consts::PI * 4.5 * 4.5 * 10_000.0;
    for i in 0..48 {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        commands.spawn((
            PointLight {
                intensity: lumens,
                range: 36.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(side * 5.5, 6.5, -4.0 - (i / 2) as f32 * 9.0),
            Visibility::Hidden,
            NightLamp(i),
        ));
    }
    // Headlight spots (vehicle_glow.rs: 60 klm, 60 m, 12 / 30 deg).
    for i in 0..8 {
        let lane = if i % 2 == 0 { -2.0 } else { 2.0 };
        let (z, dir) = if i % 2 == 0 {
            (-12.0 - i as f32 * 14.0, Vec3::NEG_Z)
        } else {
            (-45.0 - i as f32 * 10.0, Vec3::Z)
        };
        commands.spawn((
            SpotLight {
                intensity: 60_000.0,
                range: 60.0,
                inner_angle: 12f32.to_radians(),
                outer_angle: 30f32.to_radians(),
                shadow_maps_enabled: false,
                color: Color::srgb(1.0, 0.95, 0.85),
                ..default()
            },
            Transform::from_xyz(lane, 0.8, z).looking_to(dir, Vec3::Y),
            Visibility::Hidden,
            HeadSpot(i),
        ));
    }

    // Sun (SunLight in sky.rs: default DirectionalLight with shadows, default cascades).
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: 20_000.0,
            ..default()
        },
        Transform::default().looking_to(Vec3::NEG_Y, Vec3::Z),
        Sun,
    ));
}
