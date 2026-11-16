//! The hard cap on resident tiles (M-0114): the ring plus the look-ahead strips, and the farthest
//! tiles beyond it are evicted first.

use std::cmp::Reverse;

use super::{chebyshev, TileKey};

/// Tiles allowed resident: the square of the radius plus three per look-ahead step (the strip
/// of each step).
#[must_use]
pub fn resident_cap(radius: i32, ahead: i32) -> usize {
    let side = (2 * radius.max(0) + 1) as usize;
    side * side + 3 * ahead.max(0) as usize
}

/// The tiles to evict to get `resident` down to `cap`: the farthest from `centre` first, ties by
/// key. Empty while within the cap.
#[must_use]
pub fn over_cap(resident: &[TileKey], centre: TileKey, cap: usize) -> Vec<TileKey> {
    if resident.len() <= cap {
        return Vec::new();
    }
    let mut far: Vec<TileKey> = resident.to_vec();
    far.sort_by_key(|k| (Reverse(chebyshev(*k, centre)), *k));
    far.truncate(resident.len() - cap);
    far
}
