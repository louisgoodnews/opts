//! Row positions of the flattened list: prefix sums, the visible window and the reveal offset.

use std::ops::Range;

use super::{MAX_ROW_ENTITIES, OVERSCAN, Row};

/// Row positions of a flattened list: prefix sums of the row heights.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RowLayout {
    offsets: Vec<f32>,
}

impl RowLayout {
    /// The layout of `rows`.
    pub fn new(rows: &[Row]) -> Self {
        let mut offsets = Vec::with_capacity(rows.len() + 1);
        let mut y = 0.0;
        offsets.push(y);
        for r in rows {
            y += r.height();
            offsets.push(y);
        }
        Self { offsets }
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    /// Whether there are no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Total scrollable height.
    pub fn total(&self) -> f32 {
        self.offsets.last().copied().unwrap_or(0.0)
    }

    /// Top of row `i`.
    pub fn top(&self, i: usize) -> f32 {
        self.offsets.get(i).copied().unwrap_or(0.0)
    }

    /// Largest useful scroll offset.
    pub fn max_scroll(&self, viewport: f32) -> f32 {
        (self.total() - viewport).max(0.0)
    }

    /// Rows that must exist for `scroll` and `viewport` (plus [`OVERSCAN`]), never more than
    /// [`MAX_ROW_ENTITIES`].
    pub fn visible_range(&self, scroll: f32, viewport: f32) -> Range<usize> {
        let n = self.len();
        if n == 0 || viewport <= 0.0 {
            return 0..0;
        }
        let scroll = scroll.clamp(0.0, self.max_scroll(viewport));
        let first = self
            .offsets
            .partition_point(|o| *o <= scroll)
            .saturating_sub(1);
        let last = self
            .offsets
            .partition_point(|o| *o < scroll + viewport)
            .min(n);
        let start = first.saturating_sub(OVERSCAN).min(n);
        let end = (last + OVERSCAN).min(n);
        let end = end.min(start + MAX_ROW_ENTITIES);
        start..end
    }

    /// The scroll offset that brings row `i` fully into view, moving as little as possible.
    pub fn reveal(&self, i: usize, scroll: f32, viewport: f32) -> f32 {
        let top = self.top(i);
        let bottom = self.top(i + 1);
        if top < scroll {
            top
        } else if bottom > scroll + viewport {
            bottom - viewport
        } else {
            scroll
        }
    }

    /// The row under a vertical offset.
    pub fn row_at(&self, y: f32) -> usize {
        self.offsets
            .partition_point(|o| *o <= y)
            .saturating_sub(1)
            .min(self.len().saturating_sub(1))
    }
}
