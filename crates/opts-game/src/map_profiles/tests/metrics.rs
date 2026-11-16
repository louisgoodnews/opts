//! The metrics ring: five sessions per Chrono key, short sessions skipped.

use std::collections::BTreeMap;

use super::super::profile::settings_hash;

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
