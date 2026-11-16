//! Sun, traffic density and weather kinds from the clock and the presets.

use super::*;

#[test]
fn sun_and_night_follow_the_clock() {
    let noon = SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 12.0 * 3600.0, "");
    assert!(noon.sun_alt_deg() > 50.0 && !noon.night());
    assert!((noon.sun_az_deg() - 180.0).abs() < 1.0);
    let late = SessionEnvironment::new(CalendarDate::new(2026, 6, 29), 23.0 * 3600.0, "");
    assert!(late.night());
}

#[test]
fn traffic_density_reads_the_wrapped_hour_and_the_weekend() {
    use opts_core::formats::map::lists::{DensityProfile, GroupDensity};
    let group = GroupDensity {
        name: "cars".into(),
        factor: 1.0,
        profiles: vec![
            DensityProfile {
                days: day::WEEKDAY,
                points: vec![(0.0, 0.1), (8.0, 1.0), (24.0, 0.1)],
            },
            DensityProfile {
                days: day::SATURDAY | day::SUNDAY,
                points: vec![(0.0, 0.05), (24.0, 0.05)],
            },
        ],
    };
    // Friday 23:00 + 9 h = Saturday 08:00: the hour wraps (8, not 32) and the weekend curve
    // applies.
    let mut env = SessionEnvironment::new(CalendarDate::new(2026, 10, 16), 23.0 * 3600.0, "");
    assert_eq!(env.day_bit(), day::WEEKDAY);
    env.advance(9.0 * 3600.0);
    assert!((env.hour() - 8.0).abs() < 1e-4);
    assert_eq!(env.day_bit(), day::SATURDAY);
    assert!((group.density(env.day_bit(), env.hour()) - 0.05).abs() < 1e-6);
    let monday = SessionEnvironment::new(CalendarDate::new(2026, 10, 19), 8.0 * 3600.0, "");
    assert!((group.density(monday.day_bit(), monday.hour()) - 1.0).abs() < 1e-6);
}

#[test]
fn weather_kinds_from_preset_words() {
    assert_eq!(
        WeatherKind::of_preset("Clear Summer Day"),
        WeatherKind::Clear
    );
    assert_eq!(
        WeatherKind::of_preset("Overcast Autumn"),
        WeatherKind::Overcast
    );
    assert_eq!(WeatherKind::of_preset("Rainy Afternoon"), WeatherKind::Rain);
    assert_eq!(
        WeatherKind::of_preset("Autumn Morning Fog"),
        WeatherKind::Fog
    );
    assert_eq!(WeatherKind::of_preset("Winter Snowfall"), WeatherKind::Snow);
}
