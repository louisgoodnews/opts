//! Tests of the static load profile: the cache, the signature and the off-thread count.

use super::cache::cache_path;
use super::*;

fn tile(objects: usize) -> String {
    let mut t = String::from("banner\n[version]\n3\n");
    for _ in 0..objects {
        t.push_str("[object]\n0\n");
    }
    t
}

#[test]
fn the_first_answer_comes_from_a_sample_or_the_cache_and_the_full_count_is_cached() {
    let root = std::env::temp_dir().join(format!("opts-static-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let map = root.join("maps").join("M");
    std::fs::create_dir_all(&map).unwrap();
    for i in 0..6 {
        std::fs::write(map.join(format!("tile_{i}_0.map")), tile(10)).unwrap();
    }
    let store = ProfileStore::new(root.join("profiles"));
    let mut p = StaticProfiles::default();
    p.request("M", map.clone(), Some(store.clone()));
    let t = std::time::Instant::now();
    while p.get("M").is_none() && t.elapsed().as_secs() < 10 {
        p.poll();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let got = p.get("M").unwrap().clone();
    assert_eq!((got.tiles, got.objects, got.estimated), (6, 60, false));
    // The small map's full count was written; a second asker reads it back.
    for _ in 0..200 {
        if cache_path(&store, "M").exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let sig = signature(&map);
    assert_eq!(read_cache(&store, "M", sig).unwrap().objects, 60);
    // A changed tile file invalidates the cache.
    std::fs::write(map.join("tile_7_0.map"), tile(1)).unwrap();
    assert!(read_cache(&store, "M", signature(&map)).is_none());
    // Asking again for the same map does not start another count.
    p.request("M", map, Some(store));
    assert!(p.was_asked("M"));
    let _ = std::fs::remove_dir_all(&root);
}
