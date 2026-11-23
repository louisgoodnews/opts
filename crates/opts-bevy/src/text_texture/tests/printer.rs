//! Printer fonts: the glyphs of a taller alpha bitmap are read at the cells' own pixels.

use super::fixtures::*;
use super::*;

/// The 4×2 cells of `M`, `A`, `N` and the space in the alpha bitmap (rows 0-1, 2-3, 4-5, 6-7).
const M: [u8; 8] = [255, 0, 0, 255, 255, 255, 255, 255];
const A: [u8; 8] = [0, 255, 255, 0, 255, 0, 0, 255];
const N: [u8; 8] = [255, 255, 0, 0, 0, 0, 255, 255];

/// A printer font: the `M`, `A`, `N` and space cells over a black 4×2 colour bitmap, so most
/// glyph rows lie below the colour bitmap.
fn man_printer_font() -> (OftFont, FontBitmaps) {
    let chars = [('M', 0), ('A', 2), ('N', 4), (' ', 6)]
        .map(|(c, y_top)| {
            (
                c,
                OftChar {
                    x_start: 0,
                    x_end: 4,
                    y_top,
                },
            )
        })
        .into_iter()
        .collect();
    let font = OftFont {
        name: "PrinterMAN".into(),
        bitmap: String::new(),
        alpha: String::new(),
        line_height: 2,
        gap: 0,
        chars,
    };
    let bmp = FontBitmaps {
        width: 4,
        height: 2,
        colour: vec![[0, 0, 0]; 8],
        alpha_size: (4, 8),
        alpha: [M, A, N, [0; 8]].concat(),
    };
    (font, bmp)
}

#[test]
fn every_glyph_of_the_man_string_reads_back_from_its_own_cell() {
    let (font, bmp) = man_printer_font();
    // "M A N": M, space, A, space, N, one 4-px cell each, drawn at native size.
    let buf = render_rgba(&font, &bmp, &spec("M A N", 20, 2, 1));
    for (k, cell) in [M, [0; 8], A, [0; 8], N].iter().enumerate() {
        for y in 0..2 {
            let got = &alpha_row(&buf, 20, y as u32)[4 * k..4 * k + 4];
            assert_eq!(got, &cell[4 * y..4 * y + 4], "cell {k} row {y}");
        }
    }
}
