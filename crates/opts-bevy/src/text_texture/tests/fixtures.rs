//! Fixtures of the text tests: a demo font, the spec of a text.

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
