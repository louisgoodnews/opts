//! Unit tests for `super` (`nearest.rs`): the N nearest others are chosen, the player is always
//! chosen, and the distance fade reaches zero at and beyond [`FADE_END_M`].

use super::*;

#[test]
fn the_n_nearest_others_are_chosen_and_the_farther_ones_are_not() {
    // Six others with more than N = 4: the four nearest (distances 3, 5, 8, 13) and no other.
    let others = vec![
        (10, 40.0),
        (11, 13.0),
        (12, 3.0),
        (13, 60.0),
        (14, 8.0),
        (15, 5.0),
    ];
    let chosen: Vec<i32> = nearest(None, others, NEAREST_N)
        .into_iter()
        .map(|(k, _)| k)
        .collect();
    assert_eq!(chosen, vec![12, 15, 14, 11]);
}

#[test]
fn the_player_is_always_chosen_even_when_it_is_the_farthest() {
    let others = vec![(1, 2.0), (2, 3.0), (3, 4.0), (4, 5.0), (5, 6.0), (6, 7.0)];
    let chosen = nearest(Some((99, 900.0)), others, NEAREST_N);
    assert_eq!(chosen.len(), NEAREST_N + 1);
    assert_eq!(chosen[0], (99, 900.0));
    assert!(chosen.iter().all(|&(k, _)| k != 5 && k != 6));
}

#[test]
fn the_distance_fade_reaches_zero_at_and_beyond_the_stated_distance() {
    assert_eq!(fade_of(FADE_START_M), 1.0);
    assert!((fade_of((FADE_START_M + FADE_END_M) / 2.0) - 0.5).abs() < 1e-6);
    assert_eq!(fade_of(FADE_END_M), 0.0);
    assert_eq!(fade_of(FADE_END_M + 1.0), 0.0);
    assert_eq!(fade_of(1_000.0), 0.0);
}
