//! Unit tests for `super` (`maplights.rs`).

use super::*;

fn e(i: u32) -> Entity {
    Entity::from_raw_u32(i).expect("entity")
}

#[test]
fn the_nearest_n_within_the_radius_are_chosen() {
    let cands: Vec<(Entity, f32)> = (1..=10).map(|i| (e(i), i as f32 * 30.0)).collect();
    let chosen = choose_nearest(&cands, &[], 3, 200.0, 15.0);
    assert_eq!(chosen, vec![e(1), e(2), e(3)]);
    // 7 × 30 = 210 m is outside the radius.
    assert_eq!(choose_nearest(&cands, &[], 20, 200.0, 15.0).len(), 6);
}

#[test]
fn a_held_lamp_keeps_its_light_against_a_slightly_nearer_one() {
    let cands = [(e(1), 50.0), (e(2), 60.0), (e(3), 55.0)];
    // e(2) holds a light; e(3) is 5 m nearer but within the hysteresis.
    let chosen = choose_nearest(&cands, &[e(2)], 2, 200.0, 15.0);
    assert_eq!(chosen, vec![e(2), e(1)]);
    // Repeating the choice gives the same set (stable).
    let mut again = choose_nearest(&cands, &chosen, 2, 200.0, 15.0);
    again.sort();
    let mut first = chosen.clone();
    first.sort();
    assert_eq!(again, first);
    // Without the light it would lose.
    assert_eq!(
        choose_nearest(&cands, &[], 2, 200.0, 15.0),
        vec![e(1), e(3)]
    );
}

#[test]
fn slots_keep_their_lamp_and_released_ones_fade_before_reuse() {
    let mut slots: Vec<PoolSlot> = (100..102)
        .map(|i| PoolSlot {
            light: e(i),
            target: None,
            active: false,
            level: 0.0,
        })
        .collect();
    assign(&mut slots, &[e(1), e(2)]);
    assert_eq!(slots[0].target, Some(e(1)));
    assert_eq!(slots[1].target, Some(e(2)));
    slots[0].level = 1.0;
    slots[1].level = 1.0;
    // e(2) leaves, e(3) arrives: e(1) keeps its slot; e(2)'s slot fades out first.
    assign(&mut slots, &[e(1), e(3)]);
    assert_eq!(slots[0].target, Some(e(1)));
    assert!(slots[0].active);
    assert_eq!(slots[1].target, Some(e(2)));
    assert!(!slots[1].active);
    slots[1].target = None; // faded out
    assign(&mut slots, &[e(1), e(3)]);
    assert_eq!(slots[1].target, Some(e(3)));
    assert!(slots[1].active && slots[1].level == 0.0);
}

#[test]
fn photometry_follows_the_full_radius() {
    let lm = map_light_lumens(4.5, 15.0);
    // Illuminance at r: (Φ / 4π) / r² = E_full.
    let e_at_r = lm / (4.0 * std::f32::consts::PI) / (4.5 * 4.5);
    assert!((e_at_r - 15.0).abs() < 1e-3);
    assert!((map_light_range(4.5) - 36.0).abs() < 1e-6);
}
