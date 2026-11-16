use super::*;

fn utf16(text: &str) -> Vec<u8> {
    let mut v = vec![0xFF, 0xFE];
    for u in text.encode_utf16() {
        v.extend(u.to_le_bytes());
    }
    v
}

const TILE: &str = "File created with OMSI-Editor\r\n[version]\r\n3\r\n[object]\r\n0\r\nSceneryobjects\\a.sco\r\n[object]\r\n0\r\n[spline]\r\n0\r\n[spline_h]\r\n0\r\n[objects]\r\n[spline_terrain_align]\r\n1\r\n [object]\r\nx [object]\r\n";

#[test]
fn counts_records_that_stand_alone_on_a_line_in_every_encoding() {
    let want = TileCounts {
        objects: 2,
        splines: 2,
    };
    assert_eq!(count_tile(&utf16(TILE)), want);
    assert_eq!(count_tile(TILE.as_bytes()), want);
    let mut bom8 = vec![0xEF, 0xBB, 0xBF];
    bom8.extend(TILE.as_bytes());
    assert_eq!(count_tile(&bom8), want);
    // LF-only line ends and CP1252 text bytes are fine too.
    let lf = TILE.replace("\r\n", "\n").replace("a.sco", "ä.sco");
    let mut cp = lf
        .chars()
        .map(|c| if c == 'ä' { 0xE4 } else { c as u8 })
        .collect::<Vec<u8>>();
    assert_eq!(count_tile(&cp), want);
    cp.clear();
    assert_eq!(count_tile(&cp), TileCounts::default());
}

#[test]
fn a_map_that_fits_the_budget_is_counted_in_full() {
    let files: Vec<(PathBuf, u64)> = (0..10)
        .map(|i| (PathBuf::from(format!("t{i}")), 100))
        .collect();
    assert_eq!(sample_plan(&files, 5_000), ((0..10).collect(), false));
}

#[test]
fn a_big_map_is_sampled_across_its_size_mix_and_marked() {
    // 100 tiles of 1..=100 MB-ish sizes in name order unrelated to size.
    let files: Vec<(PathBuf, u64)> = (0..100)
        .map(|i| {
            (
                PathBuf::from(format!("t{i}")),
                100_000 * ((i * 37) % 100 + 1) as u64,
            )
        })
        .collect();
    let total: u64 = files.iter().map(|f| f.1).sum();
    let (picked, sampled) = sample_plan(&files, total / 5);
    assert!(sampled && picked.windows(2).all(|w| w[0] < w[1]));
    assert!(picked.len() >= 15 && picked.len() <= 25, "{}", picked.len());
    let bytes: u64 = picked.iter().map(|i| files[*i].1).sum();
    // The sample holds about a fifth of the bytes, from small and large tiles alike.
    assert!(
        (bytes as f64 / total as f64 - 0.2).abs() < 0.05,
        "{}",
        bytes as f64 / total as f64
    );
    let sizes: Vec<u64> = picked.iter().map(|i| files[*i].1).collect();
    assert!(sizes.iter().min().unwrap() < &2_000_000 && sizes.iter().max().unwrap() > &8_000_000);
}

#[test]
fn a_sampled_count_estimates_the_full_count() {
    let dir = std::env::temp_dir().join(format!("opts-count-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // 40 tiles with 5 + (i % 3) objects each and 2 splines.
    let mut full = 0;
    for i in 0..40 {
        let n = 5 + i % 3;
        full += n;
        let mut t = String::from("banner\r\n[version]\r\n3\r\n");
        for _ in 0..n {
            t.push_str("[object]\r\n0\r\n");
        }
        t.push_str("[spline]\r\n0\r\n[spline]\r\n0\r\n");
        fs::write(dir.join(format!("tile_{i}_0.map")), utf16(&t)).unwrap();
    }
    fs::write(dir.join("tile_0_0.map.terrain"), b"not a tile").unwrap();
    let exact = count_map(&dir, u64::MAX);
    assert_eq!(
        (exact.tiles, exact.objects, exact.splines, exact.sampled),
        (40, full, 80, false)
    );
    let est = count_map(&dir, 2_000);
    assert!(est.sampled && est.tiles == 40);
    let err = (est.objects as f64 - full as f64).abs() / full as f64;
    assert!(err < 0.1, "estimate {} vs {full}", est.objects);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_ai_pool_counts_distinct_vehicles_across_groups() {
    let dir = std::env::temp_dir().join(format!("opts-aipool-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let cfg = "[aigroup_2]\nNormalCars\nVehicles\\A\\a.bus\t2\nVehicles\\B\\b.bus\n[end]\n[aigroup_2]\nTrucks\nvehicles\\a\\A.bus\nVehicles\\C\\c.ovh\n[end]\n";
    fs::write(dir.join("ailists.cfg"), cfg).unwrap();
    assert_eq!(ai_pool_size(&dir), 3);
    assert_eq!(ai_pool_size(&dir.join("missing")), 0);
    let _ = fs::remove_dir_all(&dir);
}
