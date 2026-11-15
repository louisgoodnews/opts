//! Rasterising a text: the width at native size and the RGBA buffer of a rendered string.

use super::*;

/// Width in pixels of `text` drawn at native size: the glyph cells plus the font's gap between
/// neighbours (characters without a cell are skipped).
#[must_use]
pub fn text_width(font: &OftFont, text: &str) -> u32 {
    let n = text.chars().filter(|c| font.chars.contains_key(c)).count() as u32;
    cells_width(font, text) + font.gap * n.saturating_sub(1)
}

/// The glyph cells of `text` alone, without the gap: what `TextLength` gives scripts. Display
/// scripts size their `STTextOut` spacing from it (VMatrix.osc of the NG272), and their boards
/// come out right only without the gap ❓.
#[must_use]
pub fn cells_width(font: &OftFont, text: &str) -> u32 {
    text.chars()
        .filter_map(|c| font.chars.get(&c))
        .map(|g| g.x_end.saturating_sub(g.x_start))
        .sum()
}

/// Rasterises `spec.text` into an RGBA8 buffer of `spec.width × spec.height` (transparent
/// background).
#[must_use]
pub fn render_rgba(font: &OftFont, bmp: &FontBitmaps, spec: &TextSpec) -> Vec<u8> {
    let (w, h) = (spec.width.max(1), spec.height.max(1));
    let mut out = vec![0u8; (w * h * 4) as usize];
    let natural_w = text_width(font, &spec.text);
    let line_h = font.line_height.max(1);
    if natural_w == 0 {
        return out;
    }
    // Uniform shrink when the line does not fit; never enlarge.
    let scale = (w as f32 / natural_w as f32)
        .min(h as f32 / line_h as f32)
        .min(1.0);
    let drawn_w = natural_w as f32 * scale;
    let drawn_h = line_h as f32 * scale;
    let origin_x = match spec.orientation {
        1 => 0.0,
        2 => w as f32 - drawn_w,
        _ => (w as f32 - drawn_w) / 2.0,
    };
    let origin_y = (h as f32 - drawn_h) / 2.0;
    // Glyph start offsets along the line, in font pixels.
    let mut pen = 0u32;
    let mut cells = Vec::new();
    for c in spec.text.chars() {
        if let Some(g) = font.chars.get(&c) {
            cells.push((pen, *g));
            pen += g.x_end.saturating_sub(g.x_start) + font.gap;
        }
    }
    for dy in 0..h {
        let ty = (dy as f32 + 0.5 - origin_y) / scale;
        if ty < 0.0 || ty >= line_h as f32 {
            continue;
        }
        for dx in 0..w {
            let tx = (dx as f32 + 0.5 - origin_x) / scale;
            if tx < 0.0 || tx >= natural_w as f32 {
                continue;
            }
            let tx = tx as u32;
            // Last cell starting at or before tx.
            let idx = cells
                .partition_point(|(start, _)| *start <= tx)
                .saturating_sub(1);
            let (start, g) = cells[idx];
            if tx - start >= g.x_end.saturating_sub(g.x_start) {
                continue; // the gap after a glyph
            }
            let (sx, sy) = (g.x_start + (tx - start), g.y_top + ty as u32);
            if !bmp.contains(sx, sy) {
                continue;
            }
            let a = bmp.coverage(sx, sy);
            let rgb = if spec.mono {
                spec.rgb
            } else {
                bmp.colour_at(sx, sy)
            };
            let o = ((dy * w + dx) * 4) as usize;
            out[o..o + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], a]);
        }
    }
    out
}
