//! Tests of the hybrid streaming pieces (M-0114): motion estimate, object order and the
//! heading-weighted tile queue.

use super::motion::{Motion, TELEPORT_M};
use super::order::{build_order, near_first_for};
use super::policy::{effective_blend, lookahead, speed_blend, StreamMode, MAX_HEADING_WEIGHT};
use super::{evictable, wanted_tiles_weighted, Frame, TileKey};

/// Feeds a straight drive at `speed` m/s for `frames` frames at `dt` seconds per frame.
fn drive(m: &mut Motion, speed: f32, frames: usize, dt: f32) {
    let mut x = 0.0;
    for _ in 0..frames {
        x += speed * dt;
        m.observe((x, 0.0), dt);
    }
}

#[test]
fn motion_measures_a_steady_drive() {
    let mut m = Motion::default();
    drive(&mut m, 25.0, 600, 1.0 / 60.0);
    assert!(
        (m.speed_mps() - 25.0).abs() < 0.5,
        "speed {}",
        m.speed_mps()
    );
    assert!((m.heading().0 - 1.0).abs() < 1e-5 && m.heading().1.abs() < 1e-5);
}

#[test]
fn motion_stops_and_keeps_the_heading() {
    let mut m = Motion::default();
    drive(&mut m, 20.0, 600, 1.0 / 60.0);
    let last = m.heading();
    // the drive ends at x = 20 m/s * 10 s = 200 m; the vehicle then holds still there
    for _ in 0..600 {
        m.observe((200.0, 0.0), 1.0 / 60.0);
    }
    assert!(m.speed_mps() < 0.1, "speed {}", m.speed_mps());
    assert_eq!(m.heading(), last, "standing still keeps the direction");
}

#[test]
fn teleport_resets_speed_to_zero() {
    let mut m = Motion::default();
    drive(&mut m, 30.0, 120, 1.0 / 60.0);
    assert!(m.speed_mps() > 20.0);
    assert!(m.observe((1.0e5, 0.0), 1.0 / 60.0), "a jump is a teleport");
    assert_eq!(m.speed_mps(), 0.0);
    assert!(!m.observe((1.0e5 + TELEPORT_M * 0.5, 0.0), 1.0 / 60.0));
}

#[test]
fn reversing_points_the_heading_backwards() {
    let mut m = Motion::default();
    drive(&mut m, 5.0, 60, 1.0 / 60.0);
    let mut x = 5.0;
    for _ in 0..60 {
        x -= 5.0 / 60.0;
        m.observe((x, 0.0), 1.0 / 60.0);
    }
    assert!(m.heading().0 < -0.9, "heading {:?}", m.heading());
}

#[test]
fn object_order_is_nearest_first_and_stable() {
    let pts = [(10.0, 0.0), (1.0, 0.0), (1.0, 0.0), (5.0, 0.0)];
    assert_eq!(build_order(&pts, Some((0.0, 0.0))), vec![1, 2, 3, 0]);
    assert_eq!(build_order(&pts, None), vec![0, 1, 2, 3]);
    assert!(build_order(&[], Some((0.0, 0.0))).is_empty());
}

#[test]
fn object_order_only_when_slow() {
    assert!(near_first_for(0.0));
    assert!(!near_first_for(0.5));
    assert!(!near_first_for(1.0));
    assert!(near_first_for(speed_blend(0.0)));
    assert!(!near_first_for(effective_blend(StreamMode::Tile, 0.0)));
}

#[test]
fn heading_pulls_the_tile_ahead_before_one_behind_at_speed() {
    // Tile (0,0) resident, camera in its centre heading east, ring radius 1, look-ahead 2.
    let frame = Frame {
        centre: (0, 0),
        radius: 1,
        cam: (0.5, 0.5),
        forward: (1.0, 0.0),
        ahead: 2,
        weight: MAX_HEADING_WEIGHT,
    };
    let w = wanted_tiles_weighted(frame, |k| k == (0, 0), |_| true);
    assert_eq!(w[0], (1, 0), "{w:?}");
    // the look-ahead tiles come after the ring and lie east of it
    let ring = 8;
    assert!(w[ring..]
        .iter()
        .all(|k| k.0 >= 2 && (-1..=1).contains(&k.1)));
    assert_eq!(w.len(), ring + 6);
}

#[test]
fn zero_weight_is_plain_distance_order() {
    // with the heading weight zero, a tile behind at the same distance is no later than ahead
    let frame = Frame {
        centre: (0, 0),
        radius: 1,
        cam: (0.5, 0.5),
        forward: (1.0, 0.0),
        ahead: 0,
        weight: 0.0,
    };
    let w = wanted_tiles_weighted(frame, |k| k == (0, 0), |_| true);
    // the tile centre is k + 0.5 and the camera sits at 0.5, so the distance is |k| in tile units
    let dist = |k: &TileKey| (k.0 as f32).hypot(k.1 as f32);
    for pair in w.windows(2) {
        assert!(dist(&pair[0]) <= dist(&pair[1]) + 1e-4, "{w:?}");
    }
}

#[test]
fn look_ahead_tiles_survive_eviction() {
    let ahead = lookahead(1.0);
    let hyst = super::policy::hysteresis(ahead);
    // a tile two steps ahead of the ring (radius 1) must not be evicted at full tile regime
    let far: Vec<TileKey> = evictable(vec![(3, 0)], (0, 0), 1, hyst);
    assert!(far.is_empty());
    let gone: Vec<TileKey> = evictable(vec![(4, 0)], (0, 0), 1, hyst);
    assert_eq!(gone, vec![(4, 0)]);
}

#[test]
fn resident_cap_is_the_ring_plus_the_strips() {
    assert_eq!(super::resident_cap(1, 0), 9);
    assert_eq!(super::resident_cap(1, 2), 15);
    assert_eq!(super::resident_cap(2, 1), 28);
}

#[test]
fn over_cap_evicts_the_farthest_first() {
    let res = vec![(0, 0), (5, 0), (1, 0), (-3, 0)];
    assert!(super::over_cap(&res, (0, 0), 4).is_empty());
    assert_eq!(super::over_cap(&res, (0, 0), 2), vec![(5, 0), (-3, 0)]);
}
