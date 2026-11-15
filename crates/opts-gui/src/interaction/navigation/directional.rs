//! Directional focus steps: the nearest candidate in a direction, with the cost of crossing regions.

use bevy::prelude::*;

use crate::actions::Direction;

use super::{Candidate, next_linear};

/// Best candidate in `dir` from `current`: the nearest centre beyond the current one, with
/// sideways offset weighted double so rows and columns stay straight.
pub fn pick_directional(
    current: Rect,
    others: &[(Entity, Rect)],
    dir: Direction,
) -> Option<Entity> {
    let weighted: Vec<(Entity, Rect, f32)> = others.iter().map(|(e, r)| (*e, *r, 0.0)).collect();
    pick_directional_weighted(current, &weighted, dir)
}

/// [`pick_directional`] with an extra cost per candidate (logical pixels added to its distance).
pub fn pick_directional_weighted(
    current: Rect,
    others: &[(Entity, Rect, f32)],
    dir: Direction,
) -> Option<Entity> {
    let here = current.center();
    others
        .iter()
        .filter_map(|(entity, rect, penalty)| {
            let d = rect.center() - here;
            let (along, across) = match dir {
                Direction::Right => (d.x, d.y.abs()),
                Direction::Left => (-d.x, d.y.abs()),
                Direction::Down => (d.y, d.x.abs()),
                Direction::Up => (-d.y, d.x.abs()),
            };
            (along > 1.0).then_some((*entity, along + 2.0 * across + penalty))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(entity, _)| entity)
}

/// What leaving the current region costs a directional move, in logical pixels of distance: a
/// control of the same region wins unless one of another region is clearly nearer in the
/// direction pressed.
const REGION_CROSSING_COST: f32 = 200.0;

/// The target of a [`Direction`] move: geometric when the layout has run, otherwise by layout
/// order (right/down = next, left/up = previous). Prefers the current region (another region's
/// controls cost 200 logical px of extra distance), but **crosses into another region when that is
/// clearly the nearer target or the region has none in that direction**, so a gamepad or a wheel's
/// D-pad (which have no `Tab`) can reach the rail from the content and back (M-0065).
pub fn move_focus(
    candidates: &[Candidate],
    current: Option<Entity>,
    dir: Direction,
) -> Option<Entity> {
    let Some(cur) = current.and_then(|c| candidates.iter().find(|k| k.entity == c)) else {
        return next_linear(
            candidates,
            None,
            matches!(dir, Direction::Up | Direction::Left),
        );
    };
    let backwards = matches!(dir, Direction::Up | Direction::Left);
    let Some(rect) = cur.rect else {
        return next_linear(candidates, current, backwards);
    };
    let others: Vec<(Entity, Rect, f32)> = candidates
        .iter()
        .filter(|k| k.entity != cur.entity)
        .filter_map(|k| {
            let penalty = if k.region == cur.region {
                0.0
            } else {
                REGION_CROSSING_COST
            };
            k.rect.map(|r| (k.entity, r, penalty))
        })
        .collect();
    pick_directional_weighted(rect, &others, dir)
}
