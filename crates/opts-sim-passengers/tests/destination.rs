//! Destinations: the admit rule, the weighted pick and the draw at a stop.

use opts_sim_passengers::passengers::*;

fn dest(line: &str, title: &str) -> Destination {
    Destination::new(line, title)
}

#[test]
fn a_matching_line_and_text_boards() {
    let a = dest("13N", "Zoo");
    assert!(admits(Some(&a), Some(&dest("13N", "Zoo"))));
}

#[test]
fn another_line_does_not_board() {
    let a = dest("13N", "Zoo");
    assert!(!admits(Some(&a), Some(&dest("X34", "Zoo"))));
}

#[test]
fn the_same_line_with_another_text_does_not_board() {
    let a = dest("13N", "Zoo");
    assert!(!admits(Some(&a), Some(&dest("13N", "Rathaus"))));
}

#[test]
fn a_passenger_without_destination_never_boards() {
    let a = dest("13N", "Zoo");
    assert!(!admits(None, Some(&a)));
    assert!(!admits(None, None));
}

#[test]
fn a_free_drive_boards_a_passenger_whose_destination_the_stop_serves() {
    let mut t = DestinationTable::default();
    t.add(7, dest("13N", "Zoo"), 1);
    t.add(7, dest("X34", "Rathaus"), 1);
    let mut rng = Rng::new(3);
    let waiting = t.draw(7, &mut rng).expect("the stop serves a destination");
    assert!(
        admits(Some(&waiting), None),
        "a free drive boards {waiting:?}"
    );
    assert!(admits(Some(&dest("X34", "Rathaus")), None));
}

#[test]
fn a_free_drive_leaves_behind_a_passenger_at_a_stop_without_trips() {
    let t = DestinationTable::default();
    let mut rng = Rng::new(3);
    let waiting = t.draw(9, &mut rng);
    assert_eq!(waiting, None);
    assert!(!admits(waiting.as_ref(), None));
}

#[test]
fn the_pick_follows_the_weights_in_order() {
    let options = [(dest("A", "a"), 1), (dest("B", "b"), 3)];
    assert_eq!(pick(&options, 0), Some(&dest("A", "a")));
    assert_eq!(pick(&options, 1), Some(&dest("B", "b")));
    assert_eq!(pick(&options, 3), Some(&dest("B", "b")));
    assert_eq!(pick(&options, 4), None);
}

#[test]
fn equal_destinations_add_their_weights() {
    let mut t = DestinationTable::default();
    t.add(7, dest("A", "a"), 2);
    t.add(7, dest("A", "a"), 1);
    t.add(7, dest("B", "b"), 0);
    assert_eq!(t.options(7), &[(dest("A", "a"), 3)]);
}

#[test]
fn a_stop_without_trips_draws_nobody() {
    let t = DestinationTable::default();
    let mut rng = Rng::new(1);
    assert!(t.options(9).is_empty());
    assert_eq!(t.draw(9, &mut rng), None);
}

#[test]
fn the_draw_follows_the_counts() {
    let mut t = DestinationTable::default();
    t.add(7, dest("A", "a"), 1);
    t.add(7, dest("B", "b"), 3);
    let mut rng = Rng::new(42);
    let n = 4000;
    let b = (0..n)
        .filter(|_| t.draw(7, &mut rng) == Some(dest("B", "b")))
        .count();
    let share = b as f64 / f64::from(n);
    assert!((0.70..0.80).contains(&share), "share {share}");
}
