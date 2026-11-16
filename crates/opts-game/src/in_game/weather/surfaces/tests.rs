//! Unit tests of the wet and snowy surfaces: the ramp, the hold and the shader's uniform.

use super::*;
use opts_bevy::WeatherCatalogue;

fn env_of(id: &str) -> SessionEnvironment {
    let mut env = SessionEnvironment::default();
    env.apply(&opts_bevy::EnvironmentChange {
        weather: Some(id.into()),
        ..default()
    });
    env
}

#[test]
fn wetness_ramps_without_a_pop() {
    // One 60 Hz frame moves at most 0.5 / 60; a full shower takes 2 s to arrive.
    let step = next_wetness(0.0, 1.0, 1.0 / 60.0);
    assert!(step > 0.0 && step <= 0.5 / 60.0 + 1e-6, "{step}");
    let mut w = 0.0;
    for _ in 0..120 {
        let next = next_wetness(w, 1.0, 1.0 / 60.0);
        assert!((next - w).abs() <= 0.5 / 60.0 + 1e-6);
        w = next;
    }
    assert!((w - 1.0).abs() < 1e-4, "{w}");
    // Drying is ramped the same way, and out-of-range targets are clamped.
    assert!((next_wetness(1.0, 0.0, 1.0) - 0.5).abs() < 1e-6);
    assert_eq!(next_wetness(0.9, 7.0, 10.0), 1.0);
}

#[test]
fn a_hold_keeps_the_wetness_and_its_release_resumes_the_ramp() {
    // Dry weather (target 0) for ten seconds at 60 Hz: a held 0.8 does not move.
    let mut w = 0.8;
    for _ in 0..600 {
        w = drive_step(w, 0.0, 1.0 / 60.0, Some(0.8));
    }
    assert_eq!(w, 0.8);
    // A held value outside 0..=1 is clamped, not ramped.
    assert_eq!(drive_step(0.5, 0.0, 1.0 / 60.0, Some(7.0)), 1.0);
    // Released: the same frames ramp down toward the dry target again.
    let mut released = 0.8;
    for _ in 0..60 {
        released = drive_step(released, 0.0, 1.0 / 60.0, None);
    }
    assert!((released - 0.3).abs() < 1e-4, "{released}");
}

#[test]
fn rain_snow_and_the_switch_set_the_shader_uniform() {
    let on = WeatherEffects::default();
    let dry = wanted_state(&env_of("Clear Summer Day"), &on);
    assert_eq!(ground_wet(&dry), Vec4::ZERO);
    let rain = ground_wet(&wanted_state(&env_of("Rainy Afternoon"), &on));
    assert!(rain.x > 0.9 && rain.y > 0.0 && rain.z == 0.0, "{rain}");
    let snow = ground_wet(&wanted_state(&env_of("Winter Snowfall"), &on));
    assert!(snow.z > 0.8, "{snow}");
    let off = WeatherEffects {
        wet_ground: false,
        ..on
    };
    assert_eq!(
        ground_wet(&wanted_state(&env_of("Rainy Afternoon"), &off)),
        Vec4::ZERO
    );
    // Built-in presets only use the catalogue's built-ins.
    assert!(!WeatherCatalogue::builtin().is_installed());
}
