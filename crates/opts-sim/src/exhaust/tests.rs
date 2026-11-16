//! Unit tests for the exhaust puff emission (`puffs_to_spawn`) and its carry and cap rules.

use super::*;

#[test]
fn a_cold_start_at_five_degrees_puffs_and_a_warm_idle_at_thirty_does_not() {
    assert!(puff_rate_per_s(5.0, 0.0) > 0.0);
    assert_eq!(puff_rate_per_s(30.0, 0.0), 0.0);
}

#[test]
fn heavy_load_shows_even_on_a_warm_day() {
    assert!(puff_rate_per_s(30.0, 0.9) > puff_rate_per_s(30.0, 0.3));
    assert_eq!(puff_rate_per_s(30.0, 1.0), MAX_PUFFS_PER_S);
}

#[test]
fn the_cold_part_falls_linearly_and_is_bounded() {
    assert_eq!(cold_factor(COLD_BELOW_C), 0.0);
    assert_eq!(cold_factor(COLD_FULL_C - 5.0), 1.0);
    let mid = cold_factor((COLD_BELOW_C + COLD_FULL_C) / 2.0);
    assert!((mid - 0.5).abs() < 1e-6);
}

#[test]
fn the_rate_is_capped_and_inputs_are_clamped() {
    assert_eq!(puff_rate_per_s(-30.0, 5.0), MAX_PUFFS_PER_S);
    assert_eq!(puff_rate_per_s(30.0, -1.0), 0.0);
}

#[test]
fn the_emitted_count_follows_the_rate_over_time() {
    let (mut carry, mut total) = (0.0_f32, 0_u32);
    let dt = 1.0 / 60.0;
    for _ in 0..600 {
        let (n, next) = puffs_to_spawn(4.0, dt, 0, u32::MAX, carry);
        total += n;
        carry = next;
    }
    // 10 s at 4 puffs per second, within one puff of the exact count.
    assert!((total as i64 - 40).abs() <= 1, "{total}");
}

#[test]
fn no_rate_emits_nothing() {
    assert_eq!(puffs_to_spawn(0.0, 1.0, 0, 10, 0.0), (0, 0.0));
    assert_eq!(puffs_to_spawn(-3.0, 1.0, 0, 10, 0.0), (0, 0.0));
}

#[test]
fn the_cap_limits_the_live_puffs() {
    // Puffs live for PUFF_LIFETIME_S; the emitter runs at full rate for 20 s.
    let cap = 8;
    let dt = 1.0 / 60.0;
    let mut ages: Vec<f32> = Vec::new();
    let mut carry = 0.0_f32;
    let mut largest = 0_u32;
    for _ in 0..(20.0 / dt) as usize {
        for age in &mut ages {
            *age += dt;
        }
        ages.retain(|age| *age < PUFF_LIFETIME_S);
        let live = ages.len() as u32;
        let (n, next) = puffs_to_spawn(MAX_PUFFS_PER_S, dt, live, cap, carry);
        carry = next;
        ages.extend(std::iter::repeat_n(0.0, n as usize));
        largest = largest.max(ages.len() as u32);
    }
    assert!(largest <= cap, "{largest} live puffs with a cap of {cap}");
    assert_eq!(largest, cap, "the cap is reached at full rate");
}

#[test]
fn a_full_cap_spawns_nothing_and_the_carry_stays_bounded() {
    let (n, carry) = puffs_to_spawn(MAX_PUFFS_PER_S, 1.0, 12, 12, 0.0);
    assert_eq!(n, 0);
    assert!(carry <= 1.0);
}
