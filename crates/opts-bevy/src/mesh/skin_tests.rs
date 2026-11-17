//! Tests of the skin entry de-duplication: a bone's repeated (vertex, weight) pairs count once.

use super::first_positive_entries;

#[test]
fn repeats_of_a_vertex_count_once() {
    let entries = [(0_u16, 0.5_f32), (0, 0.5), (0, 0.5), (1, 0.5)];
    assert_eq!(first_positive_entries(&entries), vec![(0, 0.5), (1, 0.5)]);
}

#[test]
fn zero_weight_entry_does_not_hide_a_later_positive_one() {
    let entries = [(0_u16, 0.0_f32), (0, 0.3)];
    assert_eq!(first_positive_entries(&entries), vec![(0, 0.3)]);
}

#[test]
fn negative_and_zero_weights_are_dropped() {
    let entries = [(0_u16, -0.2_f32), (1, 0.0), (2, 0.4)];
    assert_eq!(first_positive_entries(&entries), vec![(2, 0.4)]);
}

#[test]
fn distinct_vertices_all_stay() {
    let entries = [(3_u16, 0.25_f32), (4, 0.75)];
    assert_eq!(first_positive_entries(&entries), vec![(3, 0.25), (4, 0.75)]);
}
