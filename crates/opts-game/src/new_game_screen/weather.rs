//! The weather presets of the wizard and of the pause page.
//!
//! The list is the [`WeatherCatalogue`]'s: the presets of the OMSI install (named by their `.dsc`
//! in the UI language), or the five built-in ones when there is no install. The ids are the ones
//! stored in `NewGameSetup::weather_preset` and in the drive history, including the old ones
//! (`Clear Summer Day`, …), which the catalogue maps to the closest installed preset
//! (`docs/src/architecture/weather.md`). *Live weather* is not here: it is a registered not-yet
//! place of the step.

use opts_bevy::weather::BUILTIN_IDS;
use opts_bevy::{WeatherCatalogue, WeatherConditions, WeatherKind};
use opts_gui::l10n::fluent::Arg;
use opts_gui::l10n::Localization;
use opts_gui::patterns::glyphs;

/// Most presets the wizard offers as cards (a short list); the pause page lists all.
pub const MAX_WIZARD_PRESETS: usize = 9;

/// A text that is either a message key (translated, follows the language) or data text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    /// A message key.
    Key(&'static str),
    /// Text from the install, already in the UI language.
    Text(String),
}

/// One selectable preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preset {
    /// Stored id.
    pub id: String,
    /// The name.
    pub name: Label,
    /// The one-line effect.
    pub effect: Label,
    /// Icon glyph.
    pub glyph: &'static str,
}

/// Message keys of a built-in preset: name and effect.
fn builtin_keys(id: &str) -> (&'static str, &'static str) {
    match id {
        "Overcast Autumn" => ("wiz-weather-overcast", "wiz-weather-overcast-effect"),
        "Rainy Afternoon" => ("wiz-weather-rain", "wiz-weather-rain-effect"),
        "Winter Snowfall" => ("wiz-weather-snow", "wiz-weather-snow-effect"),
        "Autumn Morning Fog" => ("wiz-weather-fog", "wiz-weather-fog-effect"),
        _ => ("wiz-weather-clear", "wiz-weather-clear-effect"),
    }
}

/// The icon of a kind of weather.
#[must_use]
pub fn glyph_of(kind: WeatherKind) -> &'static str {
    match kind {
        WeatherKind::Clear => glyphs::SUNNY,
        WeatherKind::Overcast => glyphs::CLOUD,
        WeatherKind::Rain => glyphs::WATER_DROP,
        WeatherKind::Snow => glyphs::SNOW,
        WeatherKind::Fog => glyphs::FOG,
    }
}

/// The facts of a preset in one line, from its data: temperature, sky, precipitation, fog, strong
/// wind (`9 °C · Rain · Fog, 700 m visibility`).
#[must_use]
pub fn effect_line(c: &WeatherConditions, loc: &Localization) -> String {
    use opts_core::formats::weather::PrecipKind;
    let mut parts: Vec<String> = Vec::new();
    match c.precip {
        PrecipKind::Rain => parts.push(loc.t("wiz-weather-fx-rain")),
        PrecipKind::Snow => parts.push(loc.t("wiz-weather-fx-snow")),
        PrecipKind::None => {}
    }
    match c.cloud.as_ref().map(|cl| cl.coverage) {
        Some(cover) if cover >= 0.99 => parts.push(loc.t("wiz-weather-fx-overcast")),
        Some(_) => parts.push(loc.t("wiz-weather-fx-clouds")),
        None if parts.is_empty() => parts.push(loc.t("wiz-weather-fx-clear")),
        None => {}
    }
    if c.fog_visibility_m < 5000.0 {
        parts.push(loc.t_args(
            "wiz-weather-fx-fog",
            &[(
                "m".to_owned(),
                Arg::from((c.fog_visibility_m / 5.0).round() as i32 * 5),
            )],
        ));
    }
    if c.wind_speed >= 8.0 {
        parts.push(loc.t_args(
            "wiz-weather-fx-wind",
            &[("speed".to_owned(), Arg::from(c.wind_speed.round() as i32))],
        ));
    }
    loc.t_args(
        "wiz-weather-fx-line",
        &[
            ("temp".to_owned(), Arg::from(c.temperature_c.round() as i32)),
            ("effects".to_owned(), Arg::from(parts.join(" · "))),
        ],
    )
}

/// The presets to offer, in display order.
#[must_use]
pub fn presets(catalogue: &WeatherCatalogue, loc: &Localization) -> Vec<Preset> {
    catalogue
        .ids()
        .into_iter()
        .map(|id| {
            let c = catalogue.resolve(&id);
            let glyph = glyph_of(c.kind());
            match catalogue.display_name(&id, loc.locale()) {
                Some(name) => Preset {
                    effect: Label::Text(effect_line(&c, loc)),
                    name: Label::Text(name),
                    id,
                    glyph,
                },
                None => {
                    let (name, effect) = builtin_keys(&id);
                    Preset {
                        id,
                        name: Label::Key(name),
                        effect: Label::Key(effect),
                        glyph,
                    }
                }
            }
        })
        .collect()
}

/// The name of the weather with this id (an old or unknown id gives its stand-in's name).
#[must_use]
pub fn name_of(catalogue: &WeatherCatalogue, loc: &Localization, id: &str) -> String {
    catalogue
        .display_name(id, loc.locale())
        .unwrap_or_else(|| loc.t(builtin_keys(&catalogue.canonical_id(id)).0))
}

/// The id of the default preset: the first one the list offers.
#[must_use]
pub fn default_id(catalogue: &WeatherCatalogue) -> String {
    catalogue
        .ids()
        .into_iter()
        .next()
        .unwrap_or_else(|| BUILTIN_IDS[0].to_owned())
}

#[cfg(test)]
mod tests {
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
}
