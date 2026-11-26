//! The one-tile map of the loading and scene tests.

use crate::common::MiniMap;

impl MiniMap {
    /// One tile, complete.
    pub fn simple(name: &'static str) -> Self {
        Self {
            name,
            listed: vec![(0, 0)],
            written: vec![(0, 0)],
            dangling_objects: 0,
        }
    }
}
