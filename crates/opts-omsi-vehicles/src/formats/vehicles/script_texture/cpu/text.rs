//! Text output of the in-memory canvas: the glyph cells of an OFT font.

use crate::formats::oft::OftFont;

use super::CpuScriptTexture;

impl CpuScriptTexture {
    pub(super) fn text_out_inner(
        &mut self,
        x: i32,
        y: i32,
        font: &OftFont,
        text: &str,
        _alignment: u32,
        spacing: i32,
    ) {
        let [a, r, g, b] = self.current_color;
        let pixel = [r, g, b, a];
        let mut pen_x = x;

        for c in text.chars() {
            if let Some(glyph) = font.chars.get(&c) {
                let glyph_w = glyph.x_end.saturating_sub(glyph.x_start);
                let glyph_h = font.line_height.max(1);

                // Draw filled glyph area with current color
                for gy in 0..glyph_h {
                    let py = y + gy as i32;
                    if py < 0 || py >= self.height as i32 {
                        continue;
                    }
                    for gx in 0..glyph_w {
                        let px = pen_x + gx as i32;
                        if px < 0 || px >= self.width as i32 {
                            continue;
                        }
                        let offset = ((py as u32 * self.width + px as u32) * 4) as usize;
                        self.buffer[offset..offset + 4].copy_from_slice(&pixel);
                    }
                }
                pen_x += glyph_w as i32 + spacing + font.gap as i32;
            } else if c == ' ' {
                pen_x += (font.line_height / 2).max(4) as i32 + spacing;
            }
        }
        self.dirty = true;
    }
}
