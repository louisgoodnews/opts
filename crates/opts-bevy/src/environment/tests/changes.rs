//! Changes of the environment: the revision, the map date and the systems that apply them.

use super::*;

#[test]
fn changes_bump_the_revision_and_follow_the_map_date() {
    let mut env = SessionEnvironment::new(CalendarDate::new(1990, 6, 1), 43_200.0, "Clear");
    let r0 = env.revision();
    assert_eq!(
        env.apply(&EnvironmentChange {
            rate: Some(10.0),
            ..default()
        }),
        None
    );
    assert_eq!(env.revision(), r0, "a rate is not a jump");
    assert_eq!(env.rate(), 10.0);
    env.apply(&EnvironmentChange {
        seconds_of_day: Some(90_000.0),
        weather: Some("Winter Snowfall".into()),
        ..default()
    });
    assert_eq!(env.revision(), r0 + 1);
    assert!((env.seconds_of_day() - 3600.0).abs() < 1e-6);
    assert_eq!(env.weather_kind(), WeatherKind::Snow);
    assert_eq!(env.temperature_c(), -3.0);
    assert!(env.attenuation() < 1.0);
    assert!(env.night());
    // A previewed date keeps the map date; Follow moves it and announces it once.
    env.apply(&EnvironmentChange {
        date: Some(CalendarDate::new(1991, 1, 1)),
        ..default()
    });
    assert_eq!(env.map_date(), CalendarDate::new(1990, 6, 1));
    let ann = env.apply(&EnvironmentChange {
        date: Some(CalendarDate::new(1992, 2, 2)),
        map_date: MapDatePolicy::Follow,
        ..default()
    });
    assert_eq!(
        ann,
        Some(MapDateChanged {
            old: CalendarDate::new(1990, 6, 1),
            new: CalendarDate::new(1992, 2, 2)
        })
    );
    assert_eq!(
        env.apply(&EnvironmentChange {
            date: Some(CalendarDate::new(1992, 2, 2)),
            map_date: MapDatePolicy::Follow,
            ..default()
        }),
        None
    );
    let reset = EnvironmentChange::reset(CalendarDate::new(2000, 1, 1), 0.0, "x");
    assert_eq!(env.apply(&reset), None);
    assert_eq!(env.map_date(), CalendarDate::new(2000, 1, 1));
    assert_eq!(env.rate(), 1.0);
}


#[test]
fn the_systems_tick_with_virtual_time_and_apply_changes() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(EnvironmentPlugin);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(500),
    ));
    app.world_mut().write_message(EnvironmentChange {
        rate: Some(4.0),
        ..EnvironmentChange::reset(CalendarDate::new(1994, 12, 31), 100.0, "Clear")
    });
    for _ in 0..3 {
        app.update();
    }
    let env = app.world().resource::<SessionEnvironment>();
    assert_eq!(env.rate(), 4.0);
    // The first frames have no delta; the clock moved by 0.5 s × 4 per frame after that.
    assert!(env.seconds_of_day() > 100.0, "{}", env.seconds_of_day());
    app.world_mut().write_message(EnvironmentChange {
        date: Some(CalendarDate::new(1995, 1, 2)),
        map_date: MapDatePolicy::Follow,
        ..default()
    });
    app.update();
    let msgs = app.world().resource::<Messages<MapDateChanged>>();
    let mut cursor = msgs.get_cursor();
    let got: Vec<_> = cursor.read(msgs).copied().collect();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].new, CalendarDate::new(1995, 1, 2));
}

