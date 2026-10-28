//! Unit tests for `super` (`thumbnails.rs`).

use super::*;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("opts_thumbs_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("dir");
    d
}

fn picture(path: &Path, w: u32, h: u32, rgb: [u8; 3]) {
    image::RgbImage::from_pixel(w, h, image::Rgb(rgb))
        .save(path)
        .expect("write picture");
}

#[test]
fn a_picture_is_scaled_cropped_cached_and_read_back_from_the_cache() {
    let d = dir("cache");
    let src = d.join("picture.png");
    picture(&src, 640, 480, [200, 30, 30]);
    let cache = d.join("cache");
    let a = load_thumb(&cache, "map:x", &src, HERO_SIZE, DEFAULT_CAP_BYTES).expect("thumb");
    let Thumb::Pixels { width, height, .. } = &a else {
        panic!("pixels")
    };
    assert_eq!((*width, *height), HERO_SIZE);
    let files: Vec<_> = std::fs::read_dir(&cache)
        .expect("cache")
        .flatten()
        .collect();
    assert_eq!(files.len(), 1, "one small PNG is cached");
    std::fs::remove_file(&src).expect("remove source");
    let b = load_thumb(&cache, "map:x", &src, HERO_SIZE, DEFAULT_CAP_BYTES);
    assert!(
        b.is_none(),
        "without the source the cache key cannot be built"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_cache_respects_its_size_cap_and_drops_the_oldest_first() {
    let d = dir("cap");
    let cache = d.join("cache");
    std::fs::create_dir_all(&cache).expect("cache");
    for i in 0..4 {
        let f = cache.join(format!("{i}.png"));
        std::fs::write(&f, vec![0_u8; 1000]).expect("file");
        let t = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000 + i);
        std::fs::File::options()
            .append(true)
            .open(&f)
            .and_then(|h| h.set_modified(t))
            .expect("time");
    }
    prune(&cache, 2500);
    let mut left: Vec<_> = std::fs::read_dir(&cache)
        .expect("cache")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    left.sort();
    assert_eq!(left, vec!["2.png", "3.png"], "the two oldest went");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_average_colour_of_a_flat_picture_is_that_colour() {
    let d = dir("avg");
    let src = d.join("t.png");
    picture(&src, 32, 32, [10, 120, 250]);
    assert_eq!(average_colour(&src), Some([10, 120, 250]));
    assert_eq!(average_colour(&d.join("none.png")), None);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn vehicles_without_a_preview_image_have_none_and_maps_use_picture_jpg() {
    let d = dir("src");
    std::fs::create_dir_all(d.join("Vehicles/V")).expect("dir");
    std::fs::write(d.join("Vehicles/V/v.bus"), "x").expect("bus");
    assert_eq!(vehicle_preview(&d, "Vehicles/V/v.bus"), None);
    picture(&d.join("Vehicles/V/preview.png"), 4, 4, [1, 2, 3]);
    assert!(vehicle_preview(&d, "Vehicles/V/v.bus").is_some());
    std::fs::create_dir_all(d.join("maps/Grundorf")).expect("dir");
    let cat = GameCatalogue::default();
    assert_eq!(source_of("map:Grundorf", &cat, &d), None);
    picture(&d.join("maps/Grundorf/picture.jpg"), 4, 4, [1, 2, 3]);
    assert!(source_of("map:Grundorf", &cat, &d).is_some());
    let _ = std::fs::remove_dir_all(&d);
}
