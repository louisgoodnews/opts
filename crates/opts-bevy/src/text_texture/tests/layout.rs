//! Text layout: cell widths, the gap between glyphs, and alignment.

use super::fixtures::*;
use super::*;

#[test]
fn width_sums_cells_and_skips_unknown() {
    let (font, _) = demo();
    assert_eq!(text_width(&font, "AB?"), 8);
}

#[test]
fn the_fonts_gap_separates_neighbouring_glyphs() {
    // `.oft` cells are inked edge to edge; `[newfont]`'s fifth line is the gap between them.
    let (mut font, bmp) = demo();
    font.gap = 2;
    assert_eq!(text_width(&font, "AA"), 10);
    let buf = render_rgba(&font, &bmp, &spec("AA", 10, 4, 1));
    assert_eq!(
        alpha_row(&buf, 10, 1),
        [255, 255, 255, 255, 0, 0, 255, 255, 255, 255]
    );
}

#[test]
fn left_centre_right_alignment() {
    let (font, bmp) = demo();
    let left = render_rgba(&font, &bmp, &spec("A", 12, 4, 1));
    let centre = render_rgba(&font, &bmp, &spec("A", 12, 4, 0));
    let right = render_rgba(&font, &bmp, &spec("A", 12, 4, 2));
    assert_eq!(
        alpha_row(&left, 12, 1),
        [255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(
        alpha_row(&centre, 12, 1),
        [0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0]
    );
    assert_eq!(
        alpha_row(&right, 12, 1),
        [0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255]
    );
}
