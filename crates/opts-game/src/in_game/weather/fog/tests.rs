//! Unit tests for weather fog: density, horizon colour and clear-air behaviour.
use super::*;
use opts_bevy::WeatherCatalogue;

fn fog_of(cat: &WeatherCatalogue, id: &str, effects: &WeatherEffects) -> Option<DistanceFog> {
    let c = cat.resolve(id);
    desired_fog(&c, effects, [0.5; 3], 45.0, c.fog_visibility_m, None)
}

#[test]
fn clear_air_and_a_disabled_effect_draw_no_fog() {
    let cat = WeatherCatalogue::default();
    let on = WeatherEffects::default();
    assert!(fog_of(&cat, "Clear Summer Day", &on).is_none());
    assert!(fog_of(&cat, "Autumn Morning Fog", &on).is_some());
    let off = WeatherEffects { fog: false, ..on };
    assert!(fog_of(&cat, "Autumn Morning Fog", &off).is_none());
}

#[test]
fn thicker_fog_has_a_larger_density_and_the_horizon_colour() {
    let cat = WeatherCatalogue::default();
    let on = WeatherEffects::default();
    let density = |id: &str| match fog_of(&cat, id, &on).unwrap().falloff {
        FogFalloff::ExponentialSquared { density } => density,
        other => panic!("{other:?}"),
    };
    // 300 m (morning fog) against 6 km (rain).
    assert!(density("Autumn Morning Fog") > density("Rainy Afternoon") * 10.0);
    let c = cat.resolve("Autumn Morning Fog");
    let fog = desired_fog(&c, &on, [0.2, 0.2, 0.2], 45.0, c.fog_visibility_m, None).unwrap();
    let srgb = fog.color.to_linear();
    assert!((srgb.red - srgb.blue).abs() < 0.05 && srgb.red < 0.3);
}

#[test]
fn the_fog_is_written_so_that_it_shows_the_colour_the_sky_clear_colour_shows() {
    let cat = WeatherCatalogue::default();
    let on = WeatherEffects::default();
    let c = cat.resolve("Autumn Morning Fog");
    let horizon = [0.55, 0.6, 0.7];
    let shown = |tm| {
        let fog = desired_fog(&c, &on, horizon, 45.0, c.fog_visibility_m, tm).unwrap();
        fog.color.to_linear()
    };
    let plain = shown(None);
    let comp = shown(Some(Tonemapping::SomewhatBoringDisplayTransform));
    // The compensated colour goes through the tonemapper and comes out as the plain one.
    let back = crate::look::somewhat_boring([comp.red, comp.green, comp.blue]);
    for (b, p) in back.iter().zip([plain.red, plain.green, plain.blue]) {
        assert!((b - p).abs() < 0.01, "{back:?} vs {plain:?}");
    }
}
