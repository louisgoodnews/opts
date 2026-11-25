//! Residency tests: which tiles a radius or the whole map keeps resident, which it unloads, and
//! the nearest-first order of the wanted list.

use std::collections::HashSet;

use super::super::*;

#[test]
fn radius_mode_unloads_beyond_the_radius_plus_hysteresis() {
    let resident = [(0, 0), (1, 1), (2, 2), (3, 0), (-4, 0)];
    let mut gone = evict(TileMode::Radius(1), (0, 0), resident);
    gone.sort();
    assert_eq!(gone, vec![(-4, 0), (3, 0)]);
}

#[test]
fn all_mode_never_unloads_however_far_the_camera_is() {
    let resident = [(0, 0), (40, -40), (-100, 7)];
    assert!(evict(TileMode::All, (500, 500), resident).is_empty());
}

#[test]
fn radius_mode_wants_the_square_nearest_first() {
    let none = |_| false;
    let want = wanted(TileMode::Radius(1), (10, 10), [], none);
    assert_eq!(want.len(), 9);
    assert_eq!(want[0], (10, 10));
    assert!(want[1..5].iter().all(|k| chebyshev(*k, (10, 10)) == 1));
    let resident: HashSet<TileKey> = [(10, 10), (11, 10)].into();
    let want = wanted(TileMode::Radius(1), (10, 10), [], |k| resident.contains(&k));
    assert_eq!(want.len(), 7);
    assert!(!want.contains(&(10, 10)));
}

#[test]
fn all_mode_wants_every_listed_tile_that_is_not_skipped() {
    let listed: Vec<TileKey> = (0..6).flat_map(|x| (0..4).map(move |y| (x, y))).collect();
    let resident: HashSet<TileKey> = [(0, 0), (5, 3)].into();
    let want = wanted(TileMode::All, (2, 1), listed.iter().copied(), |k| {
        resident.contains(&k)
    });
    assert_eq!(want.len(), 22);
    assert_eq!(want[0], (2, 1), "nearest first");
    let far = want.last().expect("not empty");
    assert!(chebyshev(*far, (2, 1)) >= 3);
}
