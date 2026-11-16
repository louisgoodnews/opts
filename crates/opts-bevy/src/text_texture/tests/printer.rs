//! Printer fonts: a taller alpha bitmap is read at the cells' own pixels.

use super::*;
use super::fixtures::*;

#[test]
fn a_taller_alpha_bitmap_is_read_at_the_cells_own_pixels() {
    let (font, bmp) = printer_font();
    // Scaled to the colour bitmap's height the space's rows would hit `B` (2 -> 6, clamped).
    let space = render_rgba(&font, &bmp, &spec(" ", 4, 2, 1));
    assert!(alpha_row(&space, 4, 0).iter().all(|a| *a == 0));
    assert!(alpha_row(&space, 4, 1).iter().all(|a| *a == 0));
    // `B` lies below the colour bitmap and is still drawn.
    let b = render_rgba(&font, &bmp, &spec("B", 4, 2, 1));
    assert_eq!(alpha_row(&b, 4, 0), [255, 255, 255, 255]);
    assert_eq!(&b[0..3], &[200, 0, 0], "the text colour of a mono texture");
}
