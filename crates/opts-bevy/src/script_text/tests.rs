use std::collections::HashMap;

use super::*;
use crate::host_callbacks::CpuScriptTexture;
use crate::text_texture::{FontBitmaps, OftChar};

/// A font of one glyph `A`, 4 pixels wide and 3 high: the last column is not covered, the middle
/// row is covered half.
fn font() -> (OftFont, FontBitmaps) {
    let mut chars = HashMap::new();
    chars.insert(
        'A',
        OftChar {
            x_start: 0,
            x_end: 4,
            y_top: 0,
        },
    );
    let font = OftFont {
        name: "T".into(),
        bitmap: String::new(),
        alpha: String::new(),
        line_height: 3,
        gap: 0,
        chars,
    };
    let mut alpha = vec![255u8; 12];
    for row in 0..3 {
        alpha[row * 4 + 3] = 0;
    }
    for col in 0..4 {
        alpha[4 + col] = 128;
    }
    let bmp = FontBitmaps {
        width: 4,
        height: 3,
        colour: vec![[200, 100, 50]; 12],
        alpha_size: (4, 3),
        alpha,
    };
    (font, bmp)
}

fn px(canvas: &CpuScriptTexture, x: usize, y: usize) -> [u8; 4] {
    let w = canvas.width() as usize;
    let o = (y * w + x) * 4;
    canvas.raw_rgba()[o..o + 4].try_into().expect("pixel")
}

#[test]
fn glyphs_are_drawn_in_the_fonts_colours_through_its_coverage() {
    let (font, bmp) = font();
    let mut canvas = CpuScriptTexture::new(16, 8);
    canvas.set_color(255, 1, 2, 3);
    draw_text(&mut canvas, 2, 1, &font, &bmp, "A", 0);
    // Covered pixel: the font's colour, whatever colour the script set. Uncovered column and everything around: untouched.
    assert_eq!(px(&canvas, 2, 1), [200, 100, 50, 255]);
    assert_eq!(px(&canvas, 5, 1), [0, 0, 0, 0], "the uncovered column");
    assert_eq!(px(&canvas, 1, 1), [0, 0, 0, 0], "left of the glyph");
    assert_eq!(px(&canvas, 2, 4), [0, 0, 0, 0], "below the glyph");
    // Half coverage blends with what was there (nothing: half alpha).
    let half = px(&canvas, 2, 2);
    assert!((127..=129).contains(&half[3]), "{half:?}");
    assert!(canvas.is_dirty());
}

#[test]
fn the_pen_advances_by_the_cell_width_plus_the_spacing() {
    let (font, bmp) = font();
    let mut canvas = CpuScriptTexture::new(32, 8);
    draw_text(&mut canvas, 0, 0, &font, &bmp, "AA", 2);
    // The second glyph starts at 4 + 2.
    assert_eq!(px(&canvas, 6, 0)[3], 255);
    assert_eq!(px(&canvas, 4, 0)[3], 0, "the gap between the two");
}

#[test]
fn text_beyond_the_canvas_is_clipped_not_a_panic() {
    let (font, bmp) = font();
    let mut canvas = CpuScriptTexture::new(4, 2);
    draw_text(&mut canvas, -3, -1, &font, &bmp, "AAAAAAAA", 0);
    draw_text(&mut canvas, 100, 100, &font, &bmp, "A", 0);
}
