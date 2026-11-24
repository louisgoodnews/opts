//! The road grip target and its approach (weather to friction): the target follows the wetness and
//! the snow on the road, and the friction moves toward it without a step. Moved out of
//! `in_game/weather/grip.rs` (budget room in opts-game; M-0117).

use opts_game::in_game::weather::grip::{grip_target, next_road_friction};
use opts_sim::surface_grip::{self, MU_SNOW, MU_WET};
use opts_sim::vehicle::ROAD_FRICTION;

#[test]
fn the_target_follows_wetness_and_snow_on_the_road() {
    assert_eq!(grip_target(0.0, 0.0), ROAD_FRICTION);
    assert!((grip_target(1.0, 0.0) - MU_WET).abs() < 1e-6);
    assert!((grip_target(1.0, 1.0) - MU_SNOW).abs() < 1e-6);
}

#[test]
fn the_friction_follows_a_change_without_a_step() {
    let dt = 1.0 / 60.0;
    let mut mu = ROAD_FRICTION;
    let mut largest = 0.0_f32;
    for _ in 0..(2.0 / dt) as usize {
        let next = next_road_friction(mu, 1.0, 1.0, dt);
        largest = largest.max((next - mu).abs());
        mu = next;
    }
    assert!((mu - MU_SNOW).abs() < 1e-3, "{mu} after 2 s");
    assert!(largest <= surface_grip::MAX_RATE_PER_S * dt + 1e-6);
}
