//! Tests of the metrics ring, the advice heuristics and the frame-time advice.


use super::*;


fn session(chrono: &str, ms: f32, secs: f32) -> SessionMetrics {
    SessionMetrics {
        chrono: chrono.into(),
        avg_ms: ms,
        p95_ms: ms * 1.5,
        seconds: secs,
        vehicle: String::new(),
        settings_hash: settings_hash(&BTreeMap::new()),
        date: String::new(),
    }
}

#[test]
fn metrics_keep_five_sessions_per_chrono_key_and_skip_short_ones() {
    let root = dir("metrics");
    let store = ProfileStore::new(&root);
    assert!(!store
        .record_session("M", session("default", 20.0, 59.0))
        .unwrap());
    assert!(store.sessions("M").is_empty());
    for i in 0..8 {
        assert!(store
            .record_session("M", session("default", 10.0 + i as f32, 120.0))
            .unwrap());
    }
    store
        .record_session("M", session("2020", 40.0, 90.0))
        .unwrap();
    let all = store.sessions("M");
    let default: Vec<f32> = all
        .iter()
        .filter(|s| s.chrono == "default")
        .map(|s| s.avg_ms)
        .collect();
    assert_eq!(default, vec![13.0, 14.0, 15.0, 16.0, 17.0]);
    assert_eq!(all.iter().filter(|s| s.chrono == "2020").count(), 1);
    let key = MapKey {
        map_id: "M".into(),
        chrono_id: "default".into(),
    };
    assert_eq!(store.last_session(&key).unwrap().avg_ms, 17.0);
    let _ = std::fs::remove_dir_all(&root);
}

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
