//! The plan: facts, weather, time and the saved clock.

use super::*;

#[test]
fn facts_without_data_are_not_shown_and_no_na_appears_anywhere() {
    let mut app = wizard("facts");
    press(&mut app, "wizard.footer.next");
    let t = texts(&mut app);
    assert!(
        !t.iter()
            .any(|x| x.contains("N/A") || x.contains("Scrollable")),
        "{t:?}"
    );
    cleanup("facts");
}

#[test]
fn live_weather_is_visible_marked_and_has_no_numbers() {
    let mut app = wizard("live");
    press(&mut app, "wizard.stepper.when");
    assert_eq!(step(&app), PlannerStep::When);
    let live = find_entity(app.world_mut(), "wizard.when.weather.live").expect("live card");
    let w = app.world();
    assert!(w.get::<opts_gui::not_yet::NotAvailableYet>(live).is_some());
    assert!(w
        .get::<opts_gui::interaction::state::Disabled>(live)
        .is_some());
    let t = texts(&mut app);
    assert!(t.iter().any(|x| x == "Feature not available yet!"));
    assert!(!t
        .iter()
        .any(|x| x.contains("°C") || x.contains("km/h") || x.contains('%')));
    cleanup("live");
}

#[test]
fn a_fresh_plan_starts_on_the_maps_default_date() {
    use opts_game::new_game_screen::planner::{default_map_date, setup_date, today, MapEra};
    let mut app = wizard("default-date");
    assert!(!app.world().resource::<PlannerState>().date_chosen);
    // The fixture maps have no `global.cfg`: no offset, no years, so today.
    assert_eq!(
        setup_date(app.world().resource::<NewGameSetup>()),
        default_map_date(today(), &MapEra::default())
    );
    assert!(find_entity(app.world_mut(), "wizard.where.day.plus").is_some());
    cleanup("default-date");
}

#[test]
fn quick_chips_and_steppers_change_the_time_and_date() {
    let mut app = wizard("when");
    // The date is chosen on the map step (one place), the time on step When.
    let month = app.world().resource::<NewGameSetup>().month;
    press(&mut app, "wizard.where.month.minus");
    assert_eq!(
        app.world().resource::<NewGameSetup>().month,
        if month == 1 { 12 } else { month - 1 }
    );
    assert!(app.world().resource::<PlannerState>().date_chosen);
    press(&mut app, "wizard.stepper.when");
    assert!(find_entity(app.world_mut(), "wizard.when.month.minus").is_none());
    press(&mut app, "wizard.when.quick.3");
    let s = app.world().resource::<NewGameSetup>().clone();
    assert_eq!((s.hour, s.minute), (18, 0));
    press(&mut app, "wizard.when.hour.plus");
    assert_eq!(app.world().resource::<NewGameSetup>().hour, 19);
    press(&mut app, "wizard.when.weather.rainy-afternoon");
    assert_eq!(
        app.world().resource::<NewGameSetup>().weather_preset,
        "Rainy Afternoon"
    );
    cleanup("when");
}

mod clock;
