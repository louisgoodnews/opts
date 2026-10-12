//! Windowing arithmetic of a virtualised list (spike verdict in `docs/src/architecture/ui-spikes.md`).
//!
//! Only the pure core exists so far: given a fixed row height, an item count, the scroll offset and
//! the viewport height, [`ListGeometry::visible_range`] says which rows must exist as entities. The entity
//! spawning widget follows with the first screen that needs it.

use std::ops::Range;

/// Geometry of a fixed-row-height list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListGeometry {
    /// Height of one row in logical pixels (must be positive).
    pub row_height: f32,
    /// Number of items in the model.
    pub item_count: usize,
    /// Rows kept above and below the viewport so scrolling never shows a gap.
    pub overscan: usize,
}

impl ListGeometry {
    /// Total scrollable height of the list.
    pub fn content_height(&self) -> f32 {
        self.row_height * self.item_count as f32
    }

    /// Range of item indices that must be materialised for the given scroll offset and viewport.
    pub fn visible_range(&self, scroll_offset: f32, viewport_height: f32) -> Range<usize> {
        if self.item_count == 0 || self.row_height <= 0.0 || viewport_height <= 0.0 {
            return 0..0;
        }
        let max_offset = (self.content_height() - viewport_height).max(0.0);
        let offset = scroll_offset.clamp(0.0, max_offset);
        let first = (offset / self.row_height).floor() as usize;
        let last = ((offset + viewport_height) / self.row_height).ceil() as usize;
        let start = first.saturating_sub(self.overscan);
        let end = (last + self.overscan).min(self.item_count);
        start.min(end)..end
    }

    /// Scroll offset that brings item `index` fully into view, starting from `scroll_offset`.
    pub fn scroll_to_reveal(&self, index: usize, scroll_offset: f32, viewport_height: f32) -> f32 {
        let top = index as f32 * self.row_height;
        let bottom = top + self.row_height;
        if top < scroll_offset {
            top
        } else if bottom > scroll_offset + viewport_height {
            bottom - viewport_height
        } else {
            scroll_offset
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geo(n: usize) -> ListGeometry {
        ListGeometry {
            row_height: 40.0,
            item_count: n,
            overscan: 2,
        }
    }

    #[test]
    fn only_visible_rows_plus_overscan_exist() {
        let g = geo(956);
        let r = g.visible_range(0.0, 400.0);
        assert_eq!(r, 0..12);
        let r = g.visible_range(4000.0, 400.0);
        assert_eq!(r, 98..112);
        assert!(r.len() < 20, "956 items must not materialise 956 rows");
    }

    #[test]
    fn offset_is_clamped_and_empty_lists_are_empty() {
        let g = geo(10);
        assert_eq!(g.visible_range(1.0e6, 100.0), 5..10);
        assert_eq!(geo(0).visible_range(0.0, 100.0), 0..0);
        assert_eq!(g.visible_range(0.0, 0.0), 0..0);
    }

    #[test]
    fn reveal_moves_the_minimum() {
        let g = geo(100);
        assert_eq!(g.scroll_to_reveal(0, 200.0, 400.0), 0.0);
        assert_eq!(g.scroll_to_reveal(12, 0.0, 400.0), 120.0);
        assert_eq!(g.scroll_to_reveal(3, 0.0, 400.0), 0.0);
    }
}
