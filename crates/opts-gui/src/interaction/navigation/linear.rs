//! Linear and region-wise focus steps: in layout order, wrapping around.

use bevy::prelude::*;

use super::Candidate;

/// Next (or previous) candidate in layout order, wrapping around. Without a current focus the
/// first (last) one.
pub fn next_linear(
    candidates: &[Candidate],
    current: Option<Entity>,
    backwards: bool,
) -> Option<Entity> {
    if candidates.is_empty() {
        return None;
    }
    let len = candidates.len();
    let position = current.and_then(|c| candidates.iter().position(|k| k.entity == c));
    let index = match (position, backwards) {
        (Some(i), false) => (i + 1) % len,
        (Some(i), true) => (i + len - 1) % len,
        (None, false) => 0,
        (None, true) => len - 1,
    };
    Some(candidates[index].entity)
}

/// Jump to the first control of the next (previous) region. With fewer than two regions this is
/// [`next_linear`].
pub fn next_region(
    candidates: &[Candidate],
    current: Option<Entity>,
    backwards: bool,
) -> Option<Entity> {
    let mut regions: Vec<Option<Entity>> = Vec::new();
    for c in candidates {
        if !regions.contains(&c.region) {
            regions.push(c.region);
        }
    }
    if regions.len() < 2 {
        return next_linear(candidates, current, backwards);
    }
    let here = current.and_then(|c| candidates.iter().find(|k| k.entity == c).map(|k| k.region));
    let index = match (
        here.and_then(|r| regions.iter().position(|x| *x == r)),
        backwards,
    ) {
        (Some(i), false) => (i + 1) % regions.len(),
        (Some(i), true) => (i + regions.len() - 1) % regions.len(),
        (None, false) => 0,
        (None, true) => regions.len() - 1,
    };
    candidates
        .iter()
        .find(|c| c.region == regions[index])
        .map(|c| c.entity)
}
