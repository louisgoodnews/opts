//! What a map change carries over: the vehicle, its repaint, the time and the weather.

use super::*;

#[test]
fn the_vehicle_its_repaint_the_time_and_the_weather_are_carried_over() {
    let cat = catalogue();
    let r = record("MAN_SD77", "MAN SD77", "Blue");
    let p = plan(
        &inputs(&cat, Some(&r)),
        "grundorf",
        Some("dorfplatz"),
        VehicleChoice::Carry,
    )
    .expect("plan");
    assert_eq!(p.map, "Grundorf");
    assert_eq!(p.entry_point, "Dorfplatz");
    assert!(!p.entry_defaulted && !p.no_vehicle);
    assert_eq!(
        (p.vehicle_name.as_str(), p.repaint.as_str()),
        ("MAN SD77", "Blue")
    );
    assert_eq!(p.setvars, vec![("doors".to_owned(), 1)]);
    assert_eq!(
        (p.date, p.weather.as_str()),
        (CalendarDate::new(2026, 3, 14), "Rain")
    );
    assert_eq!(
        p.dropped,
        vec!["Citaro".to_owned()],
        "the other session vehicles are listed"
    );
    let mut setup = NewGameSetup::default();
    p.apply_to(&mut setup);
    assert_eq!((setup.hour, setup.minute), (8, 45));
    assert_eq!((setup.year, setup.month, setup.day), (2026, 3, 14));
    assert_eq!(setup.selected_map, "Grundorf");
    assert_eq!(setup.selected_vehicle, "MAN SD77");
    assert_eq!(setup.selected_livery, "Blue");
    assert_eq!(setup.custom_setvars.get("doors"), Some(&1));
    assert_eq!(setup.weather_preset, "Rain");
    assert!(!setup.no_vehicle);
}

#[test]
fn a_session_without_a_vehicle_stays_without_one() {
    let cat = catalogue();
    let p = plan(&inputs(&cat, None), "Grundorf", None, VehicleChoice::Carry).expect("plan");
    assert!(p.no_vehicle);
}
