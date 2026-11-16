//! Unit tests for `super` (`start_options.rs`).

use super::*;
use serde_json::json;

#[test]
fn dates_times_and_options_are_read() {
    assert_eq!(parse_date("2026-10-20"), Ok((2026, 10, 20)));
    assert!(parse_date("2026-13-01").is_err() && parse_date("tomorrow").is_err());
    assert_eq!(parse_time("07:05"), Ok((7, 5)));
    assert!(parse_time("24:00").is_err() && parse_time("7").is_err());
    assert_eq!(parse_setvar("rollo=1"), Ok(("rollo".to_owned(), 1)));
    assert!(parse_setvar("rollo").is_err() && parse_setvar("x=y").is_err());
    assert_eq!(
        parse_trip("33 Samstag:0:2"),
        Ok(("33 Samstag".to_owned(), 0, 2))
    );
    assert!(parse_trip("33:0").is_err() && parse_trip(":0:1").is_err());
}

#[test]
fn the_options_land_in_the_setup() {
    let o = StartOptions::from_json(&json!({
        "entry": "Ahlheim Hbf", "weather": "Sturm", "date": "2026-06-01", "time": "21:30",
        "setvars": {"kneeling": 1}
    }))
    .expect("parses");
    let mut s = NewGameSetup::default();
    o.apply(&mut s, None).expect("applies");
    assert_eq!(s.selected_entry_point, "Ahlheim Hbf");
    assert_eq!(s.weather_preset, "Sturm");
    assert_eq!(
        (s.year, s.month, s.day, s.hour, s.minute),
        (2026, 6, 1, 21, 30)
    );
    assert_eq!(s.custom_setvars.get("kneeling"), Some(&1));
}

#[test]
fn wrong_types_and_values_are_refused() {
    assert!(StartOptions::from_json(&json!({"entry": 3})).is_err());
    assert!(StartOptions::from_json(&json!({"setvars": [1]})).is_err());
    assert!(StartOptions::from_json(&json!({"setvars": {"a": 1.5}})).is_err());
    let o = StartOptions::from_json(&json!({"time": "25:00"})).unwrap();
    assert!(o.apply(&mut NewGameSetup::default(), None).is_err());
}
