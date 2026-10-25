use super::*;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("opts_mipcache_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    d
}

/// A fake BC1 chain of a `w`x`h` texture whose level 0 is filled with `seed`.
fn chain(w: u32, h: u32, seed: u8) -> (Vec<u8>, u32) {
    let levels = full_chain_levels(w, h);
    let base = chain_bytes(w, h, 1, 8);
    let mut data = vec![seed; base];
    data.resize(chain_bytes(w, h, levels, 8), seed.wrapping_add(1));
    (data, levels)
}

fn key(w: u32, h: u32, level0: &[u8], generator: u32) -> ChainKey {
    ChainKey::new(TextureFormat::Bc1RgbaUnormSrgb, w, h, level0, generator)
}

#[test]
fn a_miss_then_a_hit_returns_the_stored_chain() {
    let cache = ChainCache::new(dir("hit"), DEFAULT_CAP_BYTES, 1);
    let (data, levels) = chain(64, 64, 7);
    let base = &data[..chain_bytes(64, 64, 1, 8)];
    let k = key(64, 64, base, 1);
    assert!(cache.get(&k, base).is_none());
    cache.put(&k, &data, levels);
    assert_eq!(cache.get(&k, base), Some((data.clone(), levels)));
}

#[test]
fn a_changed_source_or_generator_is_a_miss() {
    let cache = ChainCache::new(dir("invalidate"), DEFAULT_CAP_BYTES, 1);
    let (data, levels) = chain(32, 32, 3);
    let base = data[..chain_bytes(32, 32, 1, 8)].to_vec();
    cache.put(&key(32, 32, &base, 1), &data, levels);
    let mut changed = base.clone();
    changed[0] ^= 0xff;
    assert!(cache.get(&key(32, 32, &changed, 1), &changed).is_none());
    let newer = ChainCache::new(cache.dir().to_path_buf(), DEFAULT_CAP_BYTES, 2);
    assert!(newer.get(&key(32, 32, &base, 2), &base).is_none());
    // Pruning with generator 2 removes the generator-1 file.
    newer.prune();
    assert!(cache.get(&key(32, 32, &base, 1), &base).is_none());
}

#[test]
fn a_corrupted_or_truncated_file_is_ignored_and_rebuilt() {
    let cache = ChainCache::new(dir("corrupt"), DEFAULT_CAP_BYTES, 1);
    let (data, levels) = chain(16, 16, 9);
    let base = &data[..chain_bytes(16, 16, 1, 8)];
    let k = key(16, 16, base, 1);
    cache.put(&k, &data, levels);
    let path = cache.dir().join(k.file_name());
    let mut bytes = fs::read(&path).expect("read");
    let last = bytes.len() - 1;
    bytes[last] ^= 0x55;
    fs::write(&path, &bytes).expect("damage");
    assert!(cache.get(&k, base).is_none(), "damaged payload accepted");
    fs::write(&path, &bytes[..HEADER + 3]).expect("truncate");
    assert!(cache.get(&k, base).is_none(), "truncated file accepted");
    cache.put(&k, &data, levels);
    assert_eq!(cache.get(&k, base), Some((data.clone(), levels)));
}

#[test]
fn pruning_drops_the_least_recently_used_files_until_the_cap_fits() {
    let d = dir("cap");
    let big = ChainCache::new(d.clone(), DEFAULT_CAP_BYTES, 1);
    let mut keys = Vec::new();
    for seed in 0..4u8 {
        let (data, levels) = chain(64, 64, seed);
        let base = data[..chain_bytes(64, 64, 1, 8)].to_vec();
        let k = key(64, 64, &base, 1);
        big.put(&k, &data, levels);
        let age =
            std::time::SystemTime::now() - std::time::Duration::from_secs(100 - u64::from(seed));
        fs::File::options()
            .write(true)
            .open(d.join(k.file_name()))
            .and_then(|f| f.set_modified(age))
            .expect("age");
        keys.push((k, base));
    }
    let one = fs::metadata(d.join(keys[0].0.file_name()))
        .expect("meta")
        .len();
    let small = ChainCache::new(d.clone(), one * 2, 1);
    assert_eq!(small.prune(), one * 2);
    assert!(!d.join(keys[0].0.file_name()).exists(), "oldest kept");
    assert!(
        !d.join(keys[1].0.file_name()).exists(),
        "second oldest kept"
    );
    assert!(d.join(keys[3].0.file_name()).exists(), "newest dropped");
}

#[test]
fn concurrent_writers_of_one_key_leave_one_valid_file() {
    let cache = ChainCache::new(dir("concurrent"), DEFAULT_CAP_BYTES, 1);
    let (data, levels) = chain(256, 256, 5);
    let base = data[..chain_bytes(256, 256, 1, 8)].to_vec();
    let k = key(256, 256, &base, 1);
    std::thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| {
                for _ in 0..10 {
                    cache.put(&k, &data, levels);
                }
            });
        }
    });
    assert_eq!(cache.get(&k, &base), Some((data.clone(), levels)));
    let leftovers = fs::read_dir(cache.dir())
        .expect("dir")
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(leftovers, 0);
}
