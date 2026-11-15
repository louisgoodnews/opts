//! Tests of the weather presets: names, effect lines and the catalogue mapping.

use super::*;

#[test]
fn without_an_install_the_five_builtin_presets_are_listed_with_message_keys() {
    let loc = Localization::new("en");
    let cat = WeatherCatalogue::default();
    let list = presets(&cat, &loc);
    assert_eq!(list.len(), 5);
    assert_eq!(list[0].id, "Clear Summer Day");
    assert_eq!(list[0].name, Label::Key("wiz-weather-clear"));
    assert_eq!(list[2].glyph, glyphs::WATER_DROP);
    assert_eq!(default_id(&cat), "Clear Summer Day");
}

#[test]
fn old_and_unknown_ids_name_their_stand_in() {
    let loc = Localization::new("en");
    let cat = WeatherCatalogue::default();
    assert_eq!(
        name_of(&cat, &loc, "Rainy Afternoon"),
        loc.t("wiz-weather-rain")
    );
    assert_eq!(
        name_of(&cat, &loc, "Thunderstorm & Gale"),
        loc.t("wiz-weather-clear")
    );
}

#[test]
fn the_effect_line_states_the_facts_of_the_data() {
    let loc = Localization::new("en");
    let cat = WeatherCatalogue::default();
    let rain = effect_line(&cat.resolve("Rainy Afternoon"), &loc);
    assert!(rain.starts_with("15 °C"), "{rain}");
    assert!(rain.contains("Rain") && rain.contains("Overcast"), "{rain}");
    let fog = effect_line(&cat.resolve("Autumn Morning Fog"), &loc);
    assert!(fog.contains("300 m"), "{fog}");
    let clear = effect_line(&cat.resolve("Clear Summer Day"), &loc);
    assert!(clear.contains("Clear sky"), "{clear}");
    let de = Localization::new("de");
    assert!(effect_line(&cat.resolve("Rainy Afternoon"), &de).contains("Regen"));
}

fn installed() -> WeatherCatalogue {
    use opts_core::formats::weather::{
        Precip, PrecipKind, PresetFile, WeatherFile, WeatherFolder, WeatherText,
    };
    let text = |n: &str| WeatherText {
        name: n.into(),
        description: String::new(),
    };
    let rain = WeatherFile {
        precip: Precip {
            kind: PrecipKind::Rain,
            intensity: 173.0,
            extra: [0.0; 3],
        },
        ..WeatherFile::default()
    };
    WeatherCatalogue::from_folder(
        "/omsi".into(),
        WeatherFolder {
            presets: vec![
                PresetFile {
                    id: "#CAVOK".into(),
                    weather: WeatherFile::default(),
                    texts: [
                        ("DEU".to_owned(), text("<Bestes Wetter>")),
                        ("ENG".to_owned(), text("<Best Weather>")),
                    ]
                    .into(),
                },
                PresetFile {
                    id: "Schmuddelwetter".into(),
                    weather: rain,
                    texts: [("DEU".to_owned(), text("Schmuddelwetter"))].into(),
                },
            ],
            ..Default::default()
        },
    )
}

#[test]
fn an_install_lists_its_presets_named_in_the_ui_language() {
    let cat = installed();
    let de = Localization::new("de");
    let list = presets(&cat, &de);
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "#CAVOK");
    assert_eq!(list[0].name, Label::Text("Bestes Wetter".into()));
    assert_eq!(list[1].glyph, glyphs::WATER_DROP);
    assert!(matches!(&list[1].effect, Label::Text(t) if t.contains("Regen")));
    let en = Localization::new("en");
    assert_eq!(
        presets(&cat, &en)[0].name,
        Label::Text("Best Weather".into())
    );
    assert_eq!(default_id(&cat), "#CAVOK");
}

#[test]
fn old_ids_from_saves_and_history_show_their_stand_in() {
    let cat = installed();
    let en = Localization::new("en");
    // The old default is the standard weather, the old rain is the drizzle preset.
    assert_eq!(name_of(&cat, &en, "Clear Summer Day"), "Best Weather");
    assert_eq!(name_of(&cat, &en, "Rainy Afternoon"), "Schmuddelwetter");
    assert_eq!(cat.canonical_id("Rainy Afternoon"), "Schmuddelwetter");
    // No installed preset is close to snow in this catalogue: the built-in stays.
    assert_eq!(
        name_of(&cat, &en, "Winter Snowfall"),
        en.t("wiz-weather-snow")
    );
    let setup = crate::new_game_screen::types::NewGameSetup {
        weather_preset: "Rainy Afternoon".into(),
        ..Default::default()
    };
    assert!(crate::new_game_screen::choice::is_chosen(
        &crate::new_game_screen::ui::PlannerAction::Weather("Schmuddelwetter".into()),
        &setup,
        "",
        &cat,
    ));
}
