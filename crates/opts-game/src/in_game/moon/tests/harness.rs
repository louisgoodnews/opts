//! The harness of the moon tests: a minimal app with the environment plugin, a fake sun and the
//! clock setter and light reader the scenarios use.

use opts_bevy::{CalendarDate, EnvironmentChange, EnvironmentPlugin};

use super::super::*;

fn fake_sun(env: Res<SessionEnvironment>, mut sun: Query<&mut DirectionalLight, With<SunLight>>) {
    for mut l in &mut sun {
        l.illuminance = if env.sun_alt_deg() > 0.0 {
            20_000.0
        } else {
            0.0
        };
    }
}

pub(super) fn app() -> App {
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

pub(super) fn set_time(app: &mut App, date: CalendarDate, hour: f32) {
    app.world_mut().write_message(EnvironmentChange::reset(
        date,
        f64::from(hour) * 3600.0,
        "Clear Summer Day",
    ));
    app.update();
}

/// (shadow-casting lights, moon illuminance, moon hidden)
pub(super) fn state(app: &mut App) -> (usize, f32, bool) {
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
