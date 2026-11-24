//! Unit tests for `super` (`script_texture.rs`).

use super::*;
use crate::formats::oft::OftChar;
use std::collections::HashMap;

#[test]
fn cpu_script_texture_draw_rect_and_pixel_sampling() {
    let mut canvas = CpuScriptTexture::new(64, 32);
    assert_eq!(canvas.width(), 64);
    assert_eq!(canvas.height(), 32);

    canvas.lock();
    assert!(canvas.is_locked());

    // Set color A=255, R=200, G=100, B=50
    canvas.set_color(255, 200, 100, 50);
    canvas.draw_rect(10, 5, 20, 15);

    // Sample inside rect
    canvas.read_pixel(15, 10);
    assert_eq!(canvas.get_r(), 200.0);
    assert_eq!(canvas.get_g(), 100.0);
    assert_eq!(canvas.get_b(), 50.0);
    assert_eq!(canvas.get_a(), 255.0);

    // Sample outside rect
    canvas.read_pixel(0, 0);
    assert_eq!(canvas.get_r(), 0.0);
    assert_eq!(canvas.get_g(), 0.0);

    // Sample out of bounds
    canvas.read_pixel(-1, 50);
    assert_eq!(canvas.get_r(), 0.0);

    canvas.unlock();
    assert!(!canvas.is_locked());
    assert!(canvas.is_dirty());
}

#[test]
fn st_filter_leaves_the_dots_sharp() {
    let mut canvas = CpuScriptTexture::new(8, 8);
    canvas.lock();
    canvas.set_color(255, 255, 160, 0);
    canvas.draw_rect(3, 3, 3, 3);
    canvas.unlock();
    let drawn = canvas.raw_rgba().to_vec();
    for _ in 0..3 {
        canvas.filter();
    }
    assert_eq!(canvas.raw_rgba(), &drawn[..]);
}

#[test]
fn cpu_script_texture_text_out() {
    let mut canvas = CpuScriptTexture::new(128, 32);
    let mut chars = HashMap::new();
    chars.insert(
        'A',
        OftChar {
            x_start: 0,
            x_end: 8,
            y_top: 0,
        },
    );
    chars.insert(
        'B',
        OftChar {
            x_start: 8,
            x_end: 16,
            y_top: 0,
        },
    );

    let font = OftFont {
        name: "TestFont".into(),
        bitmap: "font.bmp".into(),
        alpha: "font_alpha.bmp".into(),
        line_height: 12,
        gap: 1,
        chars,
    };

    canvas.set_color(255, 255, 0, 0);
    canvas.text_out(5, 5, &font, "AB", 0, 2);

    // Pen advances: 5 + 8 (A) + 2 (spacing) + 1 (gap) = 16
    // Character A should be drawn around (5..13, 5..17)
    canvas.read_pixel(6, 6);
    assert_eq!(canvas.get_r(), 255.0);
    assert_eq!(canvas.get_g(), 0.0);
}
