use opts_core::formats::timetable::{parse_busstops, parse_line, parse_trip, Timetables};

use super::*;

/// Line 33 (player allowed) runs at 08:00, 08:30 and 10:00; the depot line (not allowed) at 08:10;
/// a Saturday line at 08:20.
fn timetables() -> Timetables {
    Timetables::from_parts(
        parse_busstops(b"[busstop]\nPonttor\n1\n100\n0\n0\n\n[busstop]\nBushof\n1\n101\n0\n0\n\n"),
        vec![
            (
                "33".into(),
                parse_line(
                    b"[userallowed]\n[priority]\n1\n[newtour]\nT1\nBus\n1\n[addtrip]\n33_V\n0\n480.000\n[addtrip]\n33_V\n0\n510.000\n[addtrip]\n33_V\n0\n600.000\n",
                ),
            ),
            (
                "depot".into(),
                parse_line(b"[priority]\n1\n[newtour]\nD\nBus\n1\n[addtrip]\n33_V\n0\n490.000\n"),
            ),
            (
                "33 Samstag".into(),
                parse_line(
                    b"[userallowed]\n[priority]\n1\n[newtour]\nS1\nBus\n1\n[addtrip]\n33_V\n0\n500.000\n",
                ),
            ),
        ],
        vec![(
            "33_V".into(),
            parse_trip(b"[trip]\n\nVaals\n33\n[station_typ2]\n100\n[station_typ2]\n101\n[profile]\nstandard\n6.0\n[profile_man_arr_time]\n0\n0.0\n[profile_man_arr_time]\n1\n6.0\n"),
        )],
    )
}

#[test]
fn player_runs_departing_within_the_hour_are_offered_earliest_first() {
    let tt = timetables();
    // Monday 07:55: the 08:00 and 08:30 runs of line 33; not 10:00 (too late), not the depot run
    // (not player allowed), not the Saturday line.
    let trips = list_trips(&tt, 7 * 3600 + 55 * 60, 1);
    let times: Vec<_> = trips.iter().map(|t| t.choice.depart_text()).collect();
    assert_eq!(times, ["08:00", "08:30"]);
    let first = &trips[0];
    assert_eq!(first.choice.station_id, 100);
    assert_eq!(first.choice.stop_name, "Ponttor");
    assert_eq!(first.choice.title(), "33 → Vaals");
    assert_eq!(first.last_stop, "Bushof");
    assert_eq!(first.stops, 2);
    assert_eq!(first.choice.id(), "33:0:0");
    // On Saturday the Saturday line's run is offered too.
    let saturday = list_trips(&tt, 7 * 3600 + 55 * 60, 6);
    assert!(saturday.iter().any(|t| t.choice.line_file == "33 Samstag"));
    // At 11:00 nothing departs within the hour.
    assert!(list_trips(&tt, 11 * 3600, 1).is_empty());
}

#[test]
fn a_run_after_midnight_counts_for_a_late_clock() {
    let tt = Timetables::from_parts(
        parse_busstops(b"[busstop]\nPonttor\n1\n100\n0\n0\n\n[busstop]\nBushof\n1\n101\n0\n0\n\n"),
        vec![(
            "N1".into(),
            parse_line(b"[userallowed]\n[priority]\n1\n[newtour]\nN\nBus\n1\n[addtrip]\nN1_V\n0\n10.000\n"),
        )],
        vec![(
            "N1_V".into(),
            parse_trip(b"[trip]\n\nNacht\nN1\n[station_typ2]\n100\n[station_typ2]\n101\n[profile]\nstandard\n6.0\n[profile_man_arr_time]\n0\n0.0\n[profile_man_arr_time]\n1\n6.0\n"),
        )],
    );
    let trips = list_trips(&tt, 23 * 3600 + 30 * 60, 1);
    assert_eq!(trips.len(), 1);
    assert_eq!(trips[0].choice.depart_text(), "00:10");
}

#[test]
fn tour_names_with_short_days_limit_the_days() {
    assert!(tour_runs_on("Mo-Fr 6", 3) && !tour_runs_on("Mo-Fr 6", 6));
    assert!(tour_runs_on("Sa 4", 6) && !tour_runs_on("Sa 4", 7));
    assert!(tour_runs_on("So 2", 7) && !tour_runs_on("So 2", 1));
    assert!(tour_runs_on("Sa+So 1", 6) && tour_runs_on("Sa+So 1", 7));
    // Words that only contain the letters do not count; a name without days runs every day.
    assert!(tour_runs_on("Sonderfahrt", 3) && tour_runs_on("T1", 2));
    assert!(tour_runs_on("Samstag 3", 6) && !tour_runs_on("Samstag 3", 5));
}

#[test]
fn the_trip_step_is_skipped_unless_runs_are_offered() {
    let mut s = PlannerState {
        step: PlannerStep::When,
        ..PlannerState::default()
    };
    assert_eq!(s.next_step(), Some(PlannerStep::Review));
    assert_eq!(s.shown_steps().len(), 4);
    s.trips_offered = true;
    assert_eq!(s.next_step(), Some(PlannerStep::Trip));
    s.step = PlannerStep::Review;
    assert_eq!(s.prev_step(), Some(PlannerStep::Trip));
    s.trips_offered = false;
    assert_eq!(s.prev_step(), Some(PlannerStep::When));
}
