//! Unit tests for `super` (`pedestrians.rs`).

use super::*;

#[test]
fn passenger_curve_scales_walkers_and_wraps() {
    let curve = [(0.0, 0.2), (4.0, 0.0), (6.0, 1.0), (7.0, 1.2), (24.0, 0.2)];
    assert!((passenger_factor(&curve, 6.5) - 1.1).abs() < 1e-6);
    assert!((passenger_factor(&curve, 26.5) - passenger_factor(&curve, 2.5)).abs() < 1e-6);
    assert_eq!(passenger_factor(&[], 3.0), 1.0);
}
