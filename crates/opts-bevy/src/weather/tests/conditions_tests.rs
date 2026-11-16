//! The conditions of a preset file, the humidity and the script variables of the weather.

use super::*;

#[test]
fn conditions_from_a_file() {
    let w = WeatherFile {
        clouds: Some(CloudLayer {
            kind: "Cumulus 3".into(),
            base_m: 300.0,
        }),
        ..WeatherFile::default()
    };
    let c = WeatherConditions::from_file(&w, &clouds());
    let cl = c.cloud.unwrap();
    // The type is not in this catalogue: no texture, scattered, coverage from the digit.
    assert_eq!(cl.texture, None);
    assert_eq!(cl.cover, CloudCover::Scattered);
    assert!((cl.coverage - 0.55).abs() < 1e-6);
    let c1 = WeatherConditions::from_file(
        &WeatherFile {
            clouds: Some(CloudLayer {
                kind: "Cumulus 1".into(),
                base_m: 100.0,
            }),
            ..WeatherFile::default()
        },
        &clouds(),
    );
    assert_eq!(
        c1.cloud.as_ref().unwrap().texture.as_deref(),
        Some("Cumulus_1.tga")
    );
    assert!((c1.cloud.unwrap().coverage - 0.25).abs() < 1e-6);
}

#[test]
fn humidity_follows_the_dew_point() {
    // 15 °C with a 5 °C dew point: about 6.6 g/m³.
    assert!((absolute_humidity(15.0, 5.0) - 6.56).abs() < 0.1);
    // Saturated air at 20 °C holds about 17.3 g/m³.
    assert!((absolute_humidity(20.0, 20.0) - 17.3).abs() < 0.3);
    // A dew point above the temperature is capped.
    assert_eq!(absolute_humidity(10.0, 30.0), absolute_humidity(10.0, 10.0));
    let c = WeatherConditions::default();
    assert!((c.abs_humidity() - 6.56).abs() < 0.1);
}

#[test]
fn script_variables_of_rain_and_dry_weather() {
    let dry = WeatherConditions::default().vars();
    assert_eq!((dry.precip_type, dry.precip_rate), (0.0, 0.0));
    let rain = installed().resolve("Schmuddelwetter").vars();
    assert_eq!(rain.precip_type, 1.0);
    assert!((rain.precip_rate - 173.0 / 255.0).abs() < 1e-6);
    let snow = WeatherCatalogue::builtin()
        .resolve("Winter Snowfall")
        .vars();
    assert_eq!(snow.precip_type, 2.0);
}
