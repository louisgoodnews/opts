//! Tests of the frame budget, tile priorities and the spread queue.

use super::{evictable, wanted_tiles, FrameBudget, Spread, TileKey};
use std::time::Duration;

#[test]
fn budget_always_allows_the_first_item() {
    let mut b = FrameBudget::new(Duration::ZERO);
    assert!(b.allow());
    assert!(!b.allow());
    assert_eq!(b.started(), 1);
}

#[test]
fn budget_allows_many_cheap_items() {
    let mut b = FrameBudget::new(Duration::from_secs(60));
    assert!((0..100).all(|_| b.allow()));
}

#[test]
fn heading_breaks_ties_between_equal_distances() {
    // camera at the centre of tile (0,0), heading east
    let cam = (0.5, 0.5);
    let fwd = (1.0, 0.0);
    let w = wanted_tiles((0, 0), 1, cam, fwd, 0, |k| k == (0, 0), |_| true);
    assert_eq!(w.len(), 8);
    assert_eq!(w[0], (1, 0), "tile ahead first: {w:?}");
    assert_eq!(
        w.last().unwrap().0,
        -1,
        "a tile behind the observer last: {w:?}"
    );
}

#[test]
fn nearest_first_without_heading() {
    let w = wanted_tiles((5, 5), 1, (5.9, 5.5), (0.0, 0.0), 0, |_| false, |_| true);
    assert_eq!(w[0], (5, 5));
    assert_eq!(w.len(), 9);
}

#[test]
fn missing_tiles_are_skipped_and_prefetch_follows_the_ring() {
    let w = wanted_tiles(
        (0, 0),
        1,
        (0.5, 0.5),
        (0.0, 1.0),
        2,
        |_| false,
        |k| k.0 != 1, // column x = 1 does not exist
    );
    assert!(w.iter().all(|k| k.0 != 1));
    // ring: 6 tiles (x in -1,0), prefetch rows at y = 2 and 3: x in -1,0 (1 missing)
    assert_eq!(w.len(), 6 + 4);
    assert!(w[..6].iter().all(|k| k.1.abs() <= 1), "ring first: {w:?}");
    assert!(w[6..].iter().all(|k| k.1 >= 2), "prefetch last: {w:?}");
}

#[test]
fn eviction_has_hysteresis() {
    let res = [(0, 0), (2, 0), (3, 0), (0, -3)];
    let ev = evictable(res, (0, 0), 1, 1);
    assert_eq!(ev, vec![(3, 0), (0, -3)]);
}

#[test]
fn spread_releases_a_bounded_number_per_frame() {
    let mut s = Spread::new(2);
    (0..5).for_each(|i| s.push(i));
    assert_eq!(s.take(), vec![0, 1]);
    assert_eq!(s.take(), vec![2, 3]);
    assert_eq!(s.take(), vec![4]);
    assert!(s.is_empty());
    assert!(s.take().is_empty());
}

/// Synthetic multi-tile map: a camera crossing 20 tiles with budgeted loading never starts more
/// than `cap` tiles in a frame, and every tile on the way ends up resident.
#[test]
fn synthetic_drive_respects_the_per_frame_cap() {
    let cap = 1;
    let mut resident: std::collections::HashSet<TileKey> = std::collections::HashSet::new();
    let mut worst = 0;
    for frame in 0..400 {
        let cam = (frame as f32 * 0.05, 0.5);
        let centre = (cam.0.floor() as i32, 0);
        for k in evictable(resident.iter().copied(), centre, 1, 1) {
            resident.remove(&k);
        }
        let mut started = 0;
        for k in wanted_tiles(
            centre,
            1,
            cam,
            (1.0, 0.0),
            1,
            |k| resident.contains(&k),
            |_| true,
        ) {
            if started >= cap {
                break;
            }
            resident.insert(k);
            started += 1;
        }
        worst = worst.max(started);
    }
    assert!(worst <= cap);
    assert!(resident.contains(&(20, 0)));
}
