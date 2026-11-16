//! Unit tests for image metrics: saturation, high-frequency energy, difference and crop parsing.
use super::*;
use image::Rgb;

fn solid(w: u32, h: u32, c: [u8; 3]) -> RgbImage {
    RgbImage::from_pixel(w, h, Rgb(c))
}

#[test]
fn grey_has_no_saturation_and_no_detail() {
    let m = measure(&solid(8, 8, [128, 128, 128]));
    assert_eq!(m.saturation, 0.0);
    assert!((m.luma_mean - 128.0 / 255.0).abs() < 1e-9);
    assert!(m.luma_std < 1e-9);
    assert_eq!(m.laplacian, 0.0);
}

#[test]
fn pure_colour_is_fully_saturated() {
    let m = measure(&solid(4, 4, [255, 0, 0]));
    assert!((m.saturation - 1.0).abs() < 1e-9);
    assert!((m.luma_min - 0.2126).abs() < 1e-9);
}

#[test]
fn checkerboard_has_more_high_frequency_energy_than_a_ramp() {
    let checker = RgbImage::from_fn(16, 16, |x, y| {
        if (x + y) % 2 == 0 {
            Rgb([255, 255, 255])
        } else {
            Rgb([0, 0, 0])
        }
    });
    let ramp = RgbImage::from_fn(16, 16, |x, _| Rgb([(x * 16) as u8; 3]));
    assert!(measure(&checker).laplacian > 10.0 * measure(&ramp).laplacian);
    assert_eq!(measure(&checker).luma_min, 0.0);
}

#[test]
fn difference_of_identical_images_is_zero() {
    let a = solid(4, 4, [10, 20, 30]);
    let d = difference(&a, &a).unwrap();
    assert_eq!(d.mean_abs, 0.0);
    assert!(d.psnr.is_infinite());
}

#[test]
fn difference_of_black_and_white_is_one() {
    let d = difference(&solid(2, 2, [0; 3]), &solid(2, 2, [255; 3])).unwrap();
    assert!((d.mean_abs - 1.0).abs() < 1e-9 && (d.rmse - 1.0).abs() < 1e-9);
    assert!(difference(&solid(2, 2, [0; 3]), &solid(3, 2, [0; 3])).is_err());
}

#[test]
fn crop_parses_and_rejects_bad_input() {
    let c = Crop::parse("1, 2,3,4").unwrap();
    assert_eq!(
        c,
        Crop {
            x: 1,
            y: 2,
            w: 3,
            h: 4
        }
    );
    assert!(Crop::parse("1,2,3").is_err());
    assert!(Crop::parse("1,2,0,4").is_err());
    assert!(c.apply(&solid(3, 3, [0; 3])).is_err());
    assert_eq!(
        c.apply(&solid(10, 10, [0; 3])).unwrap().dimensions(),
        (3, 4)
    );
}
