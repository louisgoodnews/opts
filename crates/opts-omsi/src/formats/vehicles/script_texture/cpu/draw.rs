//! Pixel and rectangle drawing of the in-memory canvas.

use super::CpuScriptTexture;

impl CpuScriptTexture {
    pub(super) fn draw_rect_inner(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let min_x = x1.min(x2).max(0) as u32;
        let max_x = (x1.max(x2) as u32).min(self.width.saturating_sub(1));
        let min_y = y1.min(y2).max(0) as u32;
        let max_y = (y1.max(y2) as u32).min(self.height.saturating_sub(1));

        if min_x > max_x || min_y > max_y {
            return;
        }

        let [a, r, g, b] = self.current_color;
        let pixel = [r, g, b, a];

        for y in min_y..=max_y {
            let row_offset = (y * self.width) as usize * 4;
            for x in min_x..=max_x {
                let offset = row_offset + (x as usize * 4);
                self.buffer[offset..offset + 4].copy_from_slice(&pixel);
            }
        }
        self.dirty = true;
    }

    pub(super) fn draw_pixel_inner(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 {
            return;
        }
        let ux = x as u32;
        let uy = y as u32;
        if ux >= self.width || uy >= self.height {
            return;
        }
        let [a, r, g, b] = self.current_color;
        let offset = ((uy * self.width + ux) * 4) as usize;
        self.buffer[offset..offset + 4].copy_from_slice(&[r, g, b, a]);
        self.dirty = true;
    }

    pub(super) fn read_pixel_inner(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 {
            self.last_read = [0, 0, 0, 0];
            return;
        }
        let ux = x as u32;
        let uy = y as u32;
        if ux >= self.width || uy >= self.height {
            self.last_read = [0, 0, 0, 0];
            return;
        }
        let offset = ((uy * self.width + ux) * 4) as usize;
        let r = self.buffer[offset];
        let g = self.buffer[offset + 1];
        let b = self.buffer[offset + 2];
        let a = self.buffer[offset + 3];
        self.last_read = [a, r, g, b];
    }
}
