//! Unit tests for `super` (`text_texture.rs`).

use super::*;
use opts_core::formats::oft::OftChar;

/// Font with two 4-px-wide cells: `A` fully covered, `B` half (left two columns).
fn demo() -> (OftFont, FontBitmaps) {
    let mut chars = HashMap::new();
    chars.insert(
        'A',
        OftChar {
            x_start: 0,
            x_end: 4,
            y_top: 0,
        },
    );
    chars.insert(
        'B',
        OftChar {
            x_start: 4,
            x_end: 8,
            y_top: 0,
        },
    );
    let font = OftFont {
        name: "Demo".into(),
        bitmap: String::new(),
        alpha: String::new(),
        line_height: 4,
        gap: 0,
        chars,
    };
    let mut alpha = vec![255u8; 32];
    for y in 0..4 {
        alpha[y * 8 + 6] = 0;
        alpha[y * 8 + 7] = 0;
    }
    let bmp = FontBitmaps {
        width: 8,
        height: 4,
        colour: vec![[10, 20, 30]; 32],
        alpha_size: (8, 4),
        alpha,
    };
    (font, bmp)
}

fn spec(text: &str, w: u32, h: u32, orientation: u32) -> TextSpec {
    TextSpec {
        font: "Demo".into(),
        text: text.into(),
        width: w,
        height: h,
        mono: true,
        rgb: [200, 0, 0],
        orientation,
    }
}

fn alpha_row(buf: &[u8], w: u32, y: u32) -> Vec<u8> {
    (0..w)
        .map(|x| buf[((y * w + x) * 4 + 3) as usize])
        .collect()
}

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

#[test]
fn mono_uses_text_colour_and_colour_mode_uses_bitmap() {
    let (font, bmp) = demo();
    let mono = render_rgba(&font, &bmp, &spec("A", 4, 4, 0));
    assert_eq!(&mono[0..4], &[200, 0, 0, 255]);
    let mut s = spec("A", 4, 4, 0);
    s.mono = false;
    let col = render_rgba(&font, &bmp, &s);
    assert_eq!(&col[0..4], &[10, 20, 30, 255]);
}

#[test]
fn alpha_bitmap_cuts_the_glyph() {
    let (font, bmp) = demo();
    let buf = render_rgba(&font, &bmp, &spec("B", 4, 4, 0));
    assert_eq!(alpha_row(&buf, 4, 0), [255, 255, 0, 0]);
}

#[test]
fn long_text_shrinks_to_fit_and_empty_text_is_transparent() {
    let (font, bmp) = demo();
    let buf = render_rgba(&font, &bmp, &spec("AB", 4, 4, 1));
    // Scale 0.5: the 4-px-high line occupies the middle two rows.
    assert_eq!(alpha_row(&buf, 4, 0), [0, 0, 0, 0]);
    assert!(alpha_row(&buf, 4, 1).iter().any(|a| *a > 0));
    assert!(render_rgba(&font, &bmp, &spec("", 4, 4, 0))
        .iter()
        .all(|b| *b == 0));
}

#[test]
fn text_source_by_number_and_by_name() {
    let vars = vec!["Alexanderplatz".to_string(), "5".to_string()];
    let names = vec!["Name".to_string(), "Kind".to_string()];
    assert_eq!(
        resolve_text(0, None, &vars, &names).as_deref(),
        Some("Alexanderplatz")
    );
    assert_eq!(
        resolve_text(0, Some("kind"), &vars, &names).as_deref(),
        Some("5")
    );
    assert_eq!(resolve_text(0, Some("missing"), &vars, &names), None);
    assert_eq!(resolve_text(3, None, &vars, &names), None);
}

/// A printer font: a black 4×2 colour bitmap and a 4×6 alpha bitmap holding the glyphs. The
/// space's cell (rows 2 and 3) is empty; `B` lies in rows 4 and 5, below the colour bitmap.
fn printer_font() -> (OftFont, FontBitmaps) {
    let mut chars = std::collections::HashMap::new();
    chars.insert(
        ' ',
        OftChar {
            x_start: 0,
            x_end: 4,
            y_top: 2,
        },
    );
    chars.insert(
        'B',
        OftChar {
            x_start: 0,
            x_end: 4,
            y_top: 4,
        },
    );
    let font = OftFont {
        name: "Printer".into(),
        bitmap: String::new(),
        alpha: String::new(),
        line_height: 2,
        gap: 0,
        chars,
    };
    let mut alpha = vec![0u8; 24];
    alpha[16..24].fill(255);
    let bmp = FontBitmaps {
        width: 4,
        height: 2,
        colour: vec![[0, 0, 0]; 8],
        alpha_size: (4, 6),
        alpha,
    };
    (font, bmp)
}

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
