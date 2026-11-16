//! The catalogue's tests: the built-in presets, old ids, names and the lookup of a user's text.

use super::*;

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
