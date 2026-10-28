//! Unit tests for `super` (`road_surf.rs`).

use super::*;

fn checker() -> Arc<SurfMap> {
    // 2 × 2: +2 cm and −2 cm alternating.
    Arc::new(SurfMap::from_heights(2, 2, vec![0.02, -0.02, -0.02, 0.02]))
}

#[test]
fn a_bmp_red_channel_decodes_to_plus_minus_two_centimetres() {
    let mut img = image::RgbImage::new(2, 1);
    img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
    img.put_pixel(1, 0, image::Rgb([0, 255, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    img.write_to(&mut bytes, image::ImageFormat::Bmp)
        .expect("bmp");
    let map = SurfMap::decode(bytes.get_ref()).expect("decodes");
    assert!((map.height_at(0.25, 0.5) - 0.02).abs() < 1e-4);
    assert!((map.height_at(0.75, 0.5) + 0.02).abs() < 1e-4);
}

#[test]
fn the_map_repeats_and_interpolates() {
    let m = checker();
    assert!((m.height_at(0.25, 0.25) - 0.02).abs() < 1e-6);
    assert!((m.height_at(1.25, -0.75) - 0.02).abs() < 1e-6);
    // Between two pixels of opposite height: level.
    assert!(m.height_at(0.5, 0.25).abs() < 1e-6);
}

#[test]
fn the_top_strip_at_road_level_gives_the_unevenness() {
    let strip = TexStrip {
        x0: -3.0,
        x1: 3.0,
        z0: 0.0,
        z1: 0.0,
        u0: 0.0,
        u1: 1.0,
        vs0: 0.0,
        vs1: 0.0,
        map: Arc::new(SurfMap::from_heights(1, 1, vec![0.015])),
        period_m: None,
    };
    let kerb = TexStrip {
        x0: 3.0,
        x1: 3.1,
        z0: 0.0,
        z1: 0.15,
        ..strip.clone()
    };
    let strips = vec![strip, kerb];
    assert!((unevenness_at(&strips, 0.0, 10.0, 0.0) - 0.015).abs() < 1e-6);
    // Off the road surface (a pavement 15 cm up, no strip there at that height): level.
    assert_eq!(unevenness_at(&strips, 5.0, 10.0, 0.15), 0.0);
}

#[test]
fn the_contact_patch_smooths_ridges_shorter_than_the_tyre() {
    // Ridges about 1.1 cm apart along the road (v per metre 45 over a 2-pixel map).
    let strip = TexStrip {
        x0: -1.0,
        x1: 1.0,
        z0: 0.0,
        z1: 0.0,
        u0: 0.25,
        u1: 0.25,
        vs0: 45.0,
        vs1: 45.0,
        map: checker(),
        period_m: None,
    };
    let raw = strip.map.height_at(0.25, 0.25).abs();
    let felt = strip
        .unevenness(0.0, 0.25 / 45.0)
        .expect("on the strip")
        .abs();
    assert!(felt < raw * 0.5, "felt {felt} raw {raw}");
}

#[test]
fn surface_types_without_a_surf_get_a_synthetic_roughness() {
    let (cobbles, _) = roughness_of(2);
    let (asphalt, _) = roughness_of(0);
    assert!(cobbles > 4.0 * asphalt);
    let map = SurfMap::noise(7, cobbles);
    assert!(map.heights.iter().all(|h| h.abs() <= cobbles + 1e-6));
    assert!(map.heights.iter().any(|h| h.abs() > cobbles * 0.5));
    assert_eq!(
        map,
        SurfMap::noise(7, cobbles),
        "the same for the same seed"
    );
    // Laid in metres: the same place gives the same height wherever the texture repeats.
    let strip = TexStrip {
        x0: -2.0,
        x1: 2.0,
        z0: 0.0,
        z1: 0.0,
        u0: 0.0,
        u1: 0.0,
        vs0: 0.0,
        vs1: 0.0,
        map: Arc::new(map),
        period_m: Some(8.0),
    };
    let a = strip.unevenness(0.3, 5.0).expect("on the strip");
    let b = strip.unevenness(0.3, 13.0).expect("on the strip");
    assert!((a - b).abs() < 1e-6);
}
