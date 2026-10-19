//! Unit tests of `weather.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::catalogue::{WeatherCatalogue, BUILTIN_IDS};
use super::fog::{
    cloud_tint, effective_cover, fog_color, fog_density, fog_layer_thickness, fogginess,
    ground_fog, ground_fog_view, light_scales, sky_veil, sky_veil_clear,
    FOG_LAYER_MAX_VISIBILITY_M, FOG_MAX_SATURATION, FOG_MAX_VISIBILITY_M,
};
use super::*;
use opts_core::formats::weather::{CloudType, WeatherText};

fn clouds() -> CloudTypes {
    CloudTypes(vec![
        CloudType {
            name: "Cumulus 1".into(),
            texture: "Cumulus_1.tga".into(),
            size_m: 2000.0,
            cover: CloudCover::Scattered,
        },
        CloudType {
            name: "Overcast 1".into(),
            texture: "OVC_1.bmp".into(),
            size_m: 1000.0,
            cover: CloudCover::Overcast,
        },
    ])
}

fn preset(id: &str, w: WeatherFile, en: &str, de: &str) -> PresetFile {
    let text = |n: &str| WeatherText {
        name: n.into(),
        description: format!("{n} text"),
    };
    PresetFile {
        id: id.into(),
        weather: w,
        texts: [("ENG".to_owned(), text(en)), ("DEU".to_owned(), text(de))].into(),
    }
}

fn installed() -> WeatherCatalogue {
    let rain = WeatherFile {
        precip: opts_core::formats::weather::Precip {
            kind: PrecipKind::Rain,
            intensity: 173.0,
            extra: [0.0; 3],
        },
        clouds: Some(CloudLayer {
            kind: "Overcast 1".into(),
            base_m: 200.0,
        }),
        fog: Fog {
            visibility_m: 200.0,
            density: 1.0,
        },
        ..WeatherFile::default()
    };
    let mut presets = vec![
        preset(
            "#CAVOK",
            WeatherFile::default(),
            "Best Weather",
            "Bestes Wetter",
        ),
        preset("Schmuddelwetter", rain, "Grey and wet", "Schmuddelwetter"),
        preset(
            "Sommerlich",
            WeatherFile::default(),
            "Summerlike",
            "Sommerlich",
        ),
    ];
    presets.sort_by(|a, b| a.id.cmp(&b.id));
    WeatherCatalogue::from_folder(
        PathBuf::from("/omsi"),
        WeatherFolder {
            presets,
            clouds: clouds(),
        },
    )
}

#[test]
fn builtin_presets_keep_the_light_and_temperature_of_the_old_kinds() {
    let cat = WeatherCatalogue::builtin();
    for (id, sun, ambient, temp) in [
        ("Clear Summer Day", 1.0, 1.0, 15.0),
        ("Overcast Autumn", 0.55, 0.8, 15.0),
        ("Rainy Afternoon", 0.55, 0.8, 15.0),
        ("Winter Snowfall", 0.55, 0.8, -3.0),
        ("Autumn Morning Fog", 0.55, 0.8, 15.0),
    ] {
        let c = cat.resolve(id);
        let l = light_scales(&c);
        assert!((l.sun - sun).abs() < 1e-3, "{id} sun {}", l.sun);
        assert!(
            (l.ambient - ambient).abs() < 1e-3,
            "{id} ambient {}",
            l.ambient
        );
        assert_eq!(c.temperature_c, temp, "{id}");
        assert_eq!(c.kind(), WeatherKind::of_preset(id), "{id}");
    }
}

#[test]
fn without_an_install_the_five_ids_are_listed() {
    let cat = WeatherCatalogue::builtin();
    assert_eq!(cat.ids(), BUILTIN_IDS.map(String::from));
    assert!(!cat.is_installed());
    assert_eq!(cat.canonical_id("Rainy Afternoon"), "Rainy Afternoon");
    // An unknown id behaves like the words it contains, else like clear weather.
    assert_eq!(cat.canonical_id("Heavy snow day"), "Winter Snowfall");
    assert_eq!(cat.canonical_id("Thunderstorm & Gale"), "Clear Summer Day");
    assert_eq!(cat.canonical_id(""), "Clear Summer Day");
}

#[test]
fn old_ids_alias_to_the_closest_installed_preset() {
    let cat = installed();
    assert_eq!(cat.ids(), ["#CAVOK", "Schmuddelwetter", "Sommerlich"]);
    assert_eq!(cat.canonical_id("Clear Summer Day"), "#CAVOK");
    assert_eq!(cat.canonical_id("Rainy Afternoon"), "Schmuddelwetter");
    // No installed preset in this catalogue is close to snow or fog: the built-in one stays.
    assert_eq!(cat.canonical_id("Winter Snowfall"), "Winter Snowfall");
    // The overcast id has no alias at all.
    assert_eq!(cat.canonical_id("Overcast Autumn"), "Overcast Autumn");
    assert_eq!(cat.canonical_id("sommerlich"), "Sommerlich");
    // The resolved conditions are the installed ones.
    let c = cat.resolve("Rainy Afternoon");
    assert_eq!(c.fog_visibility_m, 200.0);
    assert!((c.precip_rate - 173.0 / 255.0).abs() < 1e-6);
    // And the built-in ones when no alias exists.
    assert_eq!(cat.resolve("Winter Snowfall").temperature_c, -3.0);
}

#[test]
fn names_follow_the_ui_language() {
    let cat = installed();
    assert_eq!(
        cat.display_name("Sommerlich", "de").as_deref(),
        Some("Sommerlich")
    );
    assert_eq!(
        cat.display_name("#CAVOK", "de").as_deref(),
        Some("Bestes Wetter")
    );
    assert_eq!(
        cat.display_name("#CAVOK", "en").as_deref(),
        Some("Best Weather")
    );
    // A language OMSI has no text for falls back to English.
    assert_eq!(
        cat.display_name("#CAVOK", "ja").as_deref(),
        Some("Best Weather")
    );
    assert!(WeatherCatalogue::builtin()
        .display_name("Clear Summer Day", "en")
        .is_none());
}

#[test]
fn find_accepts_ids_slugs_names_words_and_old_ids() {
    let cat = installed();
    assert_eq!(
        cat.find("schmuddelwetter").as_deref(),
        Some("Schmuddelwetter")
    );
    assert_eq!(cat.find("Best Weather").as_deref(), Some("#CAVOK"));
    assert_eq!(cat.find("bestes-wetter").as_deref(), Some("#CAVOK"));
    assert_eq!(cat.find("grey").as_deref(), Some("Schmuddelwetter"));
    assert_eq!(
        cat.find("rainy-afternoon").as_deref(),
        Some("Schmuddelwetter")
    );
    assert_eq!(cat.find("rain").as_deref(), Some("Schmuddelwetter"));
    assert_eq!(cat.find("clear").as_deref(), Some("#CAVOK"));
    assert_eq!(cat.find("hail"), None);
    let b = WeatherCatalogue::builtin();
    assert_eq!(b.find("rain").as_deref(), Some("Rainy Afternoon"));
    assert_eq!(
        b.find("winter-snowfall").as_deref(),
        Some("Winter Snowfall")
    );
    assert_eq!(
        b.find("autumn-morning-fog").as_deref(),
        Some("Autumn Morning Fog")
    );
}

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

mod fog_tests;

mod look_tests;
