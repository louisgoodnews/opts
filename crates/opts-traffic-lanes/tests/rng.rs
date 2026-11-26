//! Unit tests of `rng.rs`: the same seed gives the same sequence, and floats stay in `[0, 1)`.

use opts_traffic_lanes::rng::Rng;
#[test]
fn same_seed_same_sequence() {
    let (mut a, mut b) = (Rng::new(7), Rng::new(7));
    assert!((0..16).all(|_| a.next_u64() == b.next_u64()));
    assert_ne!(Rng::new(7).next_u64(), Rng::new(8).next_u64());
}

#[test]
fn floats_stay_in_unit_interval() {
    let mut r = Rng::new(1);
    assert!((0..1000)
        .map(|_| r.next_f64())
        .all(|v| (0.0..1.0).contains(&v)));
}
