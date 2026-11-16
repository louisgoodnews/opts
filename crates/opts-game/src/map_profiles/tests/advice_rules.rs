//! The advice rules: the documented tiers and the frame-time advice.

use super::super::advice::{self, tiles_to_km2};

use super::*;

fn load(objects: usize, pool: usize) -> advice::MapLoadProfile {
    advice::MapLoadProfile {
        tile_count: 100,
        object_count: objects,
        spline_count: 1000,
        ai_pool_vehicles: pool,
        map_size_km2: tiles_to_km2(100),
        ..Default::default()
    }
}

fn ids(p: &advice::MapLoadProfile, traffic: i64) -> Vec<AdviceId> {
    advise(p, traffic).iter().map(|a| a.id).collect()
}

#[test]
fn the_heuristics_follow_the_documented_tiers() {
    assert!((tiles_to_km2(100) - 9.0).abs() < 1e-4);
    assert!(ids(&load(1_095, 10), 100).contains(&AdviceId::LightMap));
    assert!(!ids(&load(41_000, 10), 100).contains(&AdviceId::GeometryAboveTypical));
    assert!(ids(&load(60_000, 10), 100).contains(&AdviceId::GeometryAboveTypical));
    let heavy = advise(&load(300_000, 10), 100);
    let high = heavy
        .iter()
        .find(|a| a.id == AdviceId::GeometryHigh)
        .unwrap();
    assert_eq!(high.preset, Some("performance"));
    assert!(high
        .suggested
        .iter()
        .any(|(k, _)| k == "gameplay.tile_radius"));
    // Every suggested key is a map-profile key with a valid value.
    let r = registry();
    for a in advise(&load(300_000, 300), 150) {
        for (k, v) in a.suggested {
            let d = r.get(&k).unwrap_or_else(|| panic!("{k} unknown"));
            assert_eq!(
                d.profile,
                crate::settings_registry::ProfileScope::Map,
                "{k}"
            );
            crate::settings_registry::normalize(d, &v).unwrap_or_else(|e| panic!("{k}: {e}"));
        }
    }
    // The AI advice needs a large pool and a high traffic setting.
    assert!(ids(&load(10_000, 250), 150).contains(&AdviceId::HighAiLoad));
    assert!(!ids(&load(10_000, 250), 80).contains(&AdviceId::HighAiLoad));
    assert!(!ids(&load(10_000, 100), 150).contains(&AdviceId::HighAiLoad));
}

#[test]
fn frame_time_advice_uses_the_last_session() {
    let mut p = load(30_000, 10);
    assert!(ids(&p, 100).contains(&AdviceId::NotPlayedYet));
    p.avg_frame_time_ms = Some(40.0);
    assert!(ids(&p, 100).contains(&AdviceId::RanSlowly));
    assert!(!ids(&p, 100).contains(&AdviceId::NotPlayedYet));
    p.avg_frame_time_ms = Some(12.0);
    assert!(ids(&p, 100).contains(&AdviceId::RanSmoothly));
    p.avg_frame_time_ms = Some(25.0);
    let v = ids(&p, 100);
    assert!(!v.contains(&AdviceId::RanSlowly) && !v.contains(&AdviceId::RanSmoothly));
}
