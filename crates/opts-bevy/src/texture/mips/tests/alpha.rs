//! Alpha in the averages: the plain average, the weighting by visible texels and cutout detection.

use super::*;

#[test]
fn alpha_is_the_plain_average_without_coverage_mode() {
    // Left half opaque, right half transparent: level 1 of a 2x1 image is the mean.
    let mut img = image(2, 1, |x, _| {
        if x == 0 {
            [255, 0, 0, 255]
        } else {
            [0, 255, 0, 0]
        }
    });
    assert!(generate_mip_chain(&mut img, MipOptions::default()));
    let data = img.data.as_ref().expect("data");
    let l1 = level_offset(2, 1, 1);
    assert!((i32::from(data[l1 + 3]) - 128).abs() <= 1);
    // Colour is the plain linear average of red and green.
    assert!(data[l1] > 150 && data[l1 + 1] > 150);
}

#[test]
fn alpha_weighting_keeps_the_colour_of_the_visible_texels() {
    let mut img = image(2, 1, |x, _| {
        if x == 0 {
            [255, 128, 0, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    let options = MipOptions {
        weight_colour_by_alpha: true,
        ..MipOptions::default()
    };
    assert!(generate_mip_chain(&mut img, options));
    let data = img.data.as_ref().expect("data");
    let l1 = level_offset(2, 1, 1);
    assert_eq!(&data[l1..l1 + 3], &[255, 128, 0]);
    assert!((i32::from(data[l1 + 3]) - 128).abs() <= 1);
}

#[test]
fn cutout_detection_separates_foliage_from_glass() {
    let foliage = image(
        8,
        8,
        |x, _| if x < 4 { [0, 90, 0, 255] } else { [0, 0, 0, 0] },
    );
    let edge = image(8, 8, |x, _| match x {
        0..=3 => [0, 90, 0, 255],
        4 => [0, 90, 0, 128],
        _ => [0, 0, 0, 0],
    });
    let glass = image(8, 8, |_, _| [50, 60, 70, 100]);
    let solid = image(8, 8, |_, _| [50, 60, 70, 255]);
    let data = |i: &Image| i.data.clone().expect("data");
    assert!(looks_like_cutout(&data(&foliage)));
    assert!(looks_like_cutout(&data(&edge)));
    assert!(!looks_like_cutout(&data(&glass)));
    assert!(!looks_like_cutout(&data(&solid)));
}
