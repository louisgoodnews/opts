use super::path::place_along;
use super::*;

#[test]
fn a_vehicle_at_its_last_stop_is_retired_after_the_delay_and_only_if_it_is_scheduled() {
    let mut s = ScheduledTraffic::default();
    s.agent_run.insert(
        AgentId(1),
        RunRef {
            line_file: "l".into(),
            tour: 0,
            run: 0,
        },
    );
    s.finish_at(AgentId(1), 100.0, 8.0);
    s.finish_at(AgentId(2), 100.0, 8.0);
    assert!(s.take_retired(107.0).is_empty());
    assert_eq!(s.take_retired(108.0), vec![AgentId(1)]);
    assert!(s.take_retired(500.0).is_empty(), "forgotten once taken");
}

#[test]
fn a_distance_along_a_route_is_found_on_its_lane() {
    let lengths = [50.0, 100.0, 30.0];
    // 20 m into the first lane, 10 m further: lane 0 at 30.
    assert_eq!(place_along(&lengths, 20.0, 10.0), Some((0, 30.0)));
    // 40 m further: past the first lane (30 m left) and 10 m into the second.
    assert_eq!(place_along(&lengths, 20.0, 40.0), Some((1, 10.0)));
    // Beyond the end: the end of the last lane.
    assert_eq!(place_along(&lengths, 0.0, 1000.0), Some((2, 30.0)));
    assert_eq!(place_along(&[], 0.0, 1.0), None);
}

#[test]
fn arrivals_are_counted_with_their_delay() {
    let mut s = ScheduledTraffic::default();
    let run = RunRef {
        line_file: "11".into(),
        tour: 0,
        run: 2,
    };
    let id = AgentId(5);
    s.agent_run.insert(id, run.clone());
    s.runs.insert(
        run,
        RunEntry {
            agent: Some(id),
            arrivals: vec![(7, 1000.0)],
            tries: 0,
        },
    );
    s.on_event(&TrafficEvent::ArrivedAtStop {
        id,
        station: 7,
        clock: 1030.0,
    });
    s.on_event(&TrafficEvent::DepartedStop {
        id,
        station: 7,
        clock: 1050.0,
    });
    s.on_event(&TrafficEvent::StopMissed { id, station: 8 });
    assert_eq!(
        (s.stats.arrivals, s.stats.departures, s.stats.missed),
        (1, 1, 1)
    );
    assert_eq!(s.mean_delay_s(), Some(30.0));
    assert!(s.run_of(id).is_some());
}
