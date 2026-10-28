//! Unit tests for `super` (`script_stimuli.rs`).

use super::*;

#[test]
fn classes_pick_the_first_state_of_each_value() {
    let keys = vec![vec![0], vec![0], vec![1], vec![0], vec![2], vec![1]];
    assert_eq!(classes(&keys), vec![0, 2, 4]);
    assert_eq!(classes(&[vec![5], vec![5]]), vec![0]);
}

#[test]
fn quantising_groups_close_values_and_keeps_nan_apart() {
    assert_eq!(quantise(0.501, 0.05), quantise(0.499, 0.05));
    assert_ne!(quantise(0.5, 0.05), quantise(0.6, 0.05));
    assert_eq!(quantise(f32::NAN, 0.05), i64::MIN);
}

#[test]
fn an_object_fails_when_fewer_than_half_of_its_stimuli_are_live() {
    assert!(!failing(0, 0, 0.5));
    assert!(failing(4, 1, 0.5));
    assert!(!failing(4, 2, 0.5));
    assert!(failing(1, 0, 0.5));
}
