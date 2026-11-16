//! Unit tests for the control environment: weather names resolve to presets and parameters are checked.
use super::*;

fn installed() -> WeatherCatalogue {
    use opts_core::formats::weather::{PresetFile, WeatherFile, WeatherFolder};
    let preset = |id: &str| PresetFile {
        id: id.into(),
        weather: WeatherFile::default(),
        texts: Default::default(),
    };
    WeatherCatalogue::from_folder(
        "/omsi".into(),
        WeatherFolder {
            presets: vec![preset("#CAVOK"), preset("Sturm"), preset("Schmuddelwetter")],
            ..default()
        },
    )
}

#[test]
fn weather_names_resolve_to_presets() {
    let b = WeatherCatalogue::default();
    let w = |name| weather_preset(&b, name);
    assert_eq!(w("rain").as_deref(), Some("Rainy Afternoon"));
    assert_eq!(w("Winter Snowfall").as_deref(), Some("Winter Snowfall"));
    assert_eq!(
        w("autumn-morning-fog").as_deref(),
        Some("Autumn Morning Fog")
    );
    assert_eq!(w("clear").as_deref(), Some("Clear Summer Day"));
    assert_eq!(w("hail"), None);
}

#[test]
fn installed_presets_and_old_ids_are_accepted() {
    let cat = installed();
    let w = |name| weather_preset(&cat, name);
    assert_eq!(w("sturm").as_deref(), Some("Sturm"));
    assert_eq!(w("#CAVOK").as_deref(), Some("#CAVOK"));
    assert_eq!(w("schmuddelwetter").as_deref(), Some("Schmuddelwetter"));
    // The old ids stand for the closest installed preset.
    assert_eq!(w("Rainy Afternoon").as_deref(), Some("Schmuddelwetter"));
    assert_eq!(w("clear").as_deref(), Some("#CAVOK"));
    let err = change_of(&json!({"seconds": 1.0, "weather": "hail"}), &cat).unwrap_err();
    assert!(
        err.message.contains("Sturm") && err.message.contains("#CAVOK"),
        "{err:?}"
    );
    let c = change_of(&json!({"seconds": 1.0, "weather": "Sturm"}), &cat).unwrap();
    assert_eq!(c.weather.as_deref(), Some("Sturm"));
}

#[test]
fn parameters_are_checked() {
    let b = WeatherCatalogue::default();
    let c = change_of(
        &json!({"seconds": 3600.0, "date": "1994-12-31", "rate": 2.0}),
        &b,
    )
    .expect("valid");
    assert_eq!(c.date, Some(CalendarDate::new(1994, 12, 31)));
    assert_eq!(c.map_date, MapDatePolicy::Follow);
    for bad in [
        json!({"seconds": -1.0}),
        json!({"seconds": 86_400.0}),
        json!({"seconds": 1.0, "date": "1994-02-30"}),
        json!({"seconds": 1.0, "weather": "hail"}),
        json!({"seconds": 1.0, "rate": 0.0}),
        json!({}),
    ] {
        assert!(change_of(&bad, &b).is_err(), "{bad}");
    }
}

#[test]
fn the_wetness_parameter_is_clamped_to_zero_and_one() {
    assert_eq!(wetness_param(&json!({})).unwrap(), None);
    assert_eq!(wetness_param(&json!({"wetness": 0.4})).unwrap(), Some(0.4));
    assert_eq!(wetness_param(&json!({"wetness": 3.0})).unwrap(), Some(1.0));
    assert_eq!(wetness_param(&json!({"wetness": -2.0})).unwrap(), Some(0.0));
    assert!(wetness_param(&json!({"wetness": "wet"})).is_err());
}
