//! The session environment section.

use super::*;

fn environment_world() -> App {
    let mut app = session_world();
    app.add_plugins(opts_bevy::EnvironmentPlugin);
    app
}

#[test]
fn the_session_environment_round_trips() {
    use opts_bevy::{CalendarDate, EnvironmentChange, MapDatePolicy, SessionEnvironment};
    let mut app = environment_world();
    {
        let mut setup = app.world_mut().resource_mut::<NewGameSetup>();
        (setup.year, setup.month, setup.day, setup.hour, setup.minute) = (1990, 3, 4, 17, 45);
        setup.weather_preset = "Clear Summer Day".into();
        let mut info = app.world_mut().resource_mut::<InGameSessionInfo>();
        (info.start_hour, info.start_minute) = (17, 45);
    }
    app.world_mut().write_message(EnvironmentChange::reset(
        CalendarDate::new(1990, 3, 4),
        17.75 * 3600.0,
        "Clear Summer Day",
    ));
    app.update();
    // The pause page changed the date (Apply), then time, weather and rate.
    app.world_mut().write_message(EnvironmentChange {
        date: Some(CalendarDate::new(1991, 7, 8)),
        map_date: MapDatePolicy::Follow,
        ..Default::default()
    });
    app.world_mut().write_message(EnvironmentChange {
        date: Some(CalendarDate::new(1991, 7, 9)),
        seconds_of_day: Some(3_723.0),
        weather: Some("Winter Snowfall".into()),
        rate: Some(5.0),
        ..Default::default()
    });
    app.update();
    let file = capture(&mut app);
    let file = SaveFile::decode(&file.encode().expect("encode")).expect("decode");

    let mut other = environment_world();
    restore(&mut other, &file);
    {
        let setup = other.world().resource::<NewGameSetup>();
        // The map loads for its map date, with the saved weather.
        assert_eq!((setup.year, setup.month, setup.day), (1991, 7, 8));
        assert_eq!((setup.hour, setup.minute), (17, 45));
        assert_eq!(setup.weather_preset, "Winter Snowfall");
    }
    // The session starts from the plan (as `setup_in_game_session` does), then the clock comes back.
    other.world_mut().write_message(EnvironmentChange::reset(
        CalendarDate::new(1991, 7, 8),
        17.75 * 3600.0,
        "Winter Snowfall",
    ));
    restore(&mut other, &file);
    other.update();
    let env = other.world().resource::<SessionEnvironment>();
    assert_eq!(env.date(), CalendarDate::new(1991, 7, 9));
    assert_eq!(env.map_date(), CalendarDate::new(1991, 7, 8));
    assert_eq!(env.hms(), (1, 2, 3));
    assert_eq!(env.weather(), "Winter Snowfall");
    // An old preset id keeps loading: it resolves to snow and frost (a built-in preset here, the
    // closest installed one with an install), and is stored as it was.
    assert_eq!(
        env.conditions().precip,
        opts_core::formats::weather::PrecipKind::Snow
    );
    assert_eq!(env.temperature_c(), -3.0);
    assert_eq!(env.rate(), 5.0);
}

#[test]
fn an_old_time_section_restores_start_plus_play_time() {
    use opts_bevy::{CalendarDate, SessionEnvironment};
    // A save written before the session environment: no date, clock, weather or rate.
    let old: sections::TimeData = serde_json::from_value(json!({
        "year": 1994, "month": 12, "day": 31,
        "start_hour": 23, "start_minute": 30, "sim_elapsed_s": 3600.0
    }))
    .expect("old time section");
    assert_eq!(old.date, None);
    assert_eq!(old.clock(), (CalendarDate::new(1995, 1, 1), 1800.0));
    let mut app = environment_world();
    let mut file = SaveFile::new(SaveMeta::default());
    file.sections = capture(&mut app).sections;
    file.sections.get_mut("time").expect("time").data = serde_json::to_value(&old).expect("json");
    restore(&mut app, &file);
    app.update();
    let env = app.world().resource::<SessionEnvironment>();
    assert_eq!(env.date(), CalendarDate::new(1995, 1, 1));
    assert_eq!(env.hms(), (0, 30, 0));
    let setup = app.world().resource::<NewGameSetup>();
    assert_eq!((setup.year, setup.month, setup.day), (1994, 12, 31));
}
