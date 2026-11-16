//! Unit test for the vehicle script texture: canvas proportions and per-pixel colouring.
use super::*;

#[test]
fn a_picture_of_the_canvas_proportions_colours_each_pixel_a_swatch_does_not() {
    // 2 x 1: red left, blue right.
    let map = ColourMap {
        width: 2,
        height: 1,
        rgba: vec![255, 0, 0, 255, 0, 0, 255, 255],
    };
    assert!(
        map.fits(4, 2) && !map.fits(8, 4),
        "at least half the canvas size"
    );
    assert!(!map.fits(4, 4), "other proportions: a swatch");
    let mut data = vec![0, 0, 0, 200, 0, 0, 0, 0, 0, 0, 0, 50, 0, 0, 0, 255];
    map.paint(&mut data, 4, 1);
    assert_eq!(&data[0..4], &[255, 0, 0, 200], "left: red, alpha kept");
    assert_eq!(&data[12..16], &[0, 0, 255, 255], "right: blue");
}
