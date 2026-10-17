//! Plain-language texts built from the choices (summary line, review card).

use opts_gui::l10n::Localization;

use super::planner::{clock_text, month_key};
use super::types::{GameCatalogue, NewGameSetup};
use opts_bevy::WeatherCatalogue;

/// `9 October 1989`.
pub fn date_text(loc: &Localization, setup: &NewGameSetup) -> String {
    loc.t_args(
        "wiz-date",
        &[
            ("day".to_string(), (setup.day as i32).into()),
            ("month".to_string(), loc.t(month_key(setup.month)).into()),
            ("year".to_string(), setup.year.into()),
        ],
    )
}

/// The name of the chosen weather.
pub fn weather_name(
    loc: &Localization,
    weather: &WeatherCatalogue,
    setup: &NewGameSetup,
) -> String {
    super::weather::name_of(weather, loc, &setup.weather_preset)
}

/// The vehicle as the user knows it.
pub fn vehicle_name(catalogue: &GameCatalogue, setup: &NewGameSetup) -> String {
    catalogue
        .find_vehicle(&setup.selected_vehicle)
        .map_or_else(|| setup.selected_vehicle.clone(), |v| v.full_name.clone())
}

/// The vehicle line of a drive: the vehicle's name, or *No vehicle* for a map-only start.
pub fn vehicle_line(loc: &Localization, catalogue: &GameCatalogue, setup: &NewGameSetup) -> String {
    if setup.no_vehicle {
        loc.t("wiz-no-vehicle")
    } else {
        vehicle_name(catalogue, setup)
    }
}

/// The livery in use: the chosen one or the vehicle's default (may be empty; always empty
/// without a vehicle).
pub fn livery_name(catalogue: &GameCatalogue, setup: &NewGameSetup) -> String {
    if setup.no_vehicle {
        return String::new();
    }
    if !setup.selected_livery.is_empty() {
        return setup.selected_livery.clone();
    }
    catalogue
        .find_vehicle(&setup.selected_vehicle)
        .map(|v| v.default_livery.clone())
        .unwrap_or_default()
}

/// Part ①: map and start point.
pub fn where_text(setup: &NewGameSetup) -> String {
    if setup.selected_entry_point.is_empty() {
        setup.selected_map.clone()
    } else {
        format!("{} · {}", setup.selected_map, setup.selected_entry_point)
    }
}

/// Part ②: vehicle and livery.
pub fn what_text(loc: &Localization, catalogue: &GameCatalogue, setup: &NewGameSetup) -> String {
    let name = vehicle_line(loc, catalogue, setup);
    let livery = livery_name(catalogue, setup);
    if livery.is_empty() {
        name
    } else {
        format!("{name} · {livery}")
    }
}

/// Part ③: date, time and weather.
pub fn when_text(loc: &Localization, weather: &WeatherCatalogue, setup: &NewGameSetup) -> String {
    format!(
        "{} {} · {}",
        date_text(loc, setup),
        clock_text(setup),
        weather_name(loc, weather, setup)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::new_game_screen::types::VehicleItem;

    #[test]
    fn texts_name_things_the_way_the_user_knows_them() {
        let loc = Localization::new("en");
        let mut cat = GameCatalogue::default();
        let mut v = VehicleItem::named("MAN_SD77", "MAN", "SD77");
        v.default_livery = "Beige".into();
        cat.vehicles.push(v);
        let s = NewGameSetup {
            selected_map: "Berlin".into(),
            selected_entry_point: "Rathaus".into(),
            selected_vehicle: "MAN SD77".into(),
            year: 1989,
            month: 10,
            day: 9,
            ..NewGameSetup::default()
        };
        assert_eq!(where_text(&s), "Berlin · Rathaus");
        assert_eq!(what_text(&loc, &cat, &s), "MAN SD77 · Beige");
        assert_eq!(date_text(&loc, &s), "9 October 1989");
        assert_eq!(
            when_text(&loc, &WeatherCatalogue::default(), &s),
            "9 October 1989 08:30 · Clear day"
        );
        let de = Localization::new("de");
        assert_eq!(date_text(&de, &s), "9. Oktober 1989");
    }
}
