//! Detail by distance: full detail up to 60 m, frozen beyond.

use opts_pedestrian_rules::lod::{detail_at, Detail, FULL_DETAIL_M};

#[test]
fn the_limit_is_sixty_metres() {
    assert_eq!(FULL_DETAIL_M, 60.0);
}

#[test]
fn walkers_within_the_limit_keep_full_detail() {
    assert_eq!(detail_at(0.0), Detail::Full);
    assert_eq!(detail_at(25.0), Detail::Full);
    // Exactly at the limit is still full.
    assert_eq!(detail_at(60.0), Detail::Full);
}

#[test]
fn walkers_beyond_the_limit_are_frozen() {
    assert_eq!(detail_at(60.5), Detail::Frozen);
    assert_eq!(detail_at(140.0), Detail::Frozen);
}

#[test]
fn a_distance_that_is_not_a_number_counts_as_near() {
    assert_eq!(detail_at(f32::NAN), Detail::Full);
}
