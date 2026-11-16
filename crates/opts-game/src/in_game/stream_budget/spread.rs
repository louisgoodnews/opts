//! Queue that spreads work (despawns) over frames.

use std::collections::VecDeque;

/// Queue that releases at most `per_frame` items per call (spreads despawns over frames).
#[derive(Debug, Clone)]
pub struct Spread<T> {
    items: VecDeque<T>,
    per_frame: usize,
}

impl<T> Spread<T> {
    /// A queue releasing `per_frame` (at least 1) items per [`Spread::take`].
    #[must_use]
    pub fn new(per_frame: usize) -> Self {
        Self {
            items: VecDeque::new(),
            per_frame: per_frame.max(1),
        }
    }

    /// Queues an item.
    pub fn push(&mut self, item: T) {
        self.items.push_back(item);
    }

    /// Items waiting.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether nothing waits.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The items this frame handles.
    pub fn take(&mut self) -> Vec<T> {
        let n = self.per_frame.min(self.items.len());
        self.items.drain(..n).collect()
    }
}
