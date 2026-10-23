use super::*;
use opts_core::formats::timetable::ScheduledStop;

fn stop(id: u32, arr: f32, dep: f32) -> ScheduledStop {
    ScheduledStop {
        station_id: id,
        name: String::new(),
        arrival_s: arr,
        departure_s: dep,
        distance_m: None,
    }
}

fn run3() -> ActiveSchedule {
    ActiveSchedule {
        line_file: "11".into(),
        tour: "t".into(),
        trip: "trip".into(),
        title: "Ziel".into(),
        line: "11".into(),
        start_s: 1000.0,
        stops: vec![
            stop(1, 1000.0, 1000.0),
            stop(2, 1100.0, 1120.0),
            stop(3, 1300.0, 1300.0),
        ],
    }
}

#[test]
fn a_run_is_before_at_between_and_after_its_stops() {
    let s = run3();
    assert_eq!(run_phase(&s, 900.0), RunPhase::NotStarted { wait_s: 100.0 });
    assert_eq!(
        run_phase(&s, 1050.0),
        RunPhase::Between {
            from: 0,
            fraction: 0.5
        }
    );
    assert_eq!(run_phase(&s, 1110.0), RunPhase::AtStop { index: 1 });
    match run_phase(&s, 1210.0) {
        RunPhase::Between { from: 1, fraction } => assert!((fraction - 0.5).abs() < 1e-9),
        other => panic!("{other:?}"),
    }
    assert_eq!(run_phase(&s, 1300.0), RunPhase::Finished);
    assert_eq!(run_phase(&s, 5000.0), RunPhase::Finished);
}

#[test]
fn an_empty_run_is_finished() {
    let mut s = run3();
    s.stops.clear();
    assert_eq!(run_phase(&s, 0.0), RunPhase::Finished);
}

#[test]
fn line_files_run_on_the_days_their_names_give() {
    assert!(line_runs_on("Eichenhoehe TA11 Mo-Do Schule", 3));
    assert!(!line_runs_on("Eichenhoehe TA11 Mo-Do Schule", 5));
    assert!(line_runs_on("Eichenhoehe TA12 Freitag Schule", 5));
    assert!(line_runs_on("Eichenhoehe TA13 Samstag", 6));
    assert!(line_runs_on("Eichenhoehe TA14 Sonn- und Feiertag", 7));
    assert!(!line_runs_on("Eichenhoehe TA14 Sonn- und Feiertag", 1));
    assert!(
        !line_runs_on("Eichenhoehe TA31 Mo-Do Ferien", 2),
        "school holidays are not run"
    );
    assert!(line_runs_on("13N", 4) && line_runs_on("Z_KI Zuege Mitte", 7));
}
