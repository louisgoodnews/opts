//! Fixtures of the text tests: a demo font, the spec of a text, and a printer font.

use super::*;

/// Font with two 4-px-wide cells: `A` fully covered, `B` half (left two columns).
pub(super) fn demo() -> (OftFont, FontBitmaps) {
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

pub(super) fn spec(text: &str, w: u32, h: u32, orientation: u32) -> TextSpec {
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

pub(super) fn alpha_row(buf: &[u8], w: u32, y: u32) -> Vec<u8> {
    (0..w)
        .map(|x| buf[((y * w + x) * 4 + 3) as usize])
        .collect()
}

/// A printer font: a black 4×2 colour bitmap and a 4×6 alpha bitmap holding the glyphs. The
/// space's cell (rows 2 and 3) is empty; `B` lies in rows 4 and 5, below the colour bitmap.
pub(super) fn printer_font() -> (OftFont, FontBitmaps) {
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
