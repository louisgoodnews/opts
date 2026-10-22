//! Boxes for a collision mesh that follow its shape.
//!
//! One box around the whole mesh is right for a bench or a stop shelter, wrong for a bridge: the
//! box of a railway bridge reaches from the ground on its abutments up to the deck, so a bus could
//! not drive under it; wrong for a street lamp too: its pole and arm in one plane made a wall
//! 3.7 m wide and 7.5 m high that stopped a bus on Köln's Frankenstraße. A mesh is cut into
//! columns on a plan grid (2 m for a bridge, finer for small things); each column keeps only the
//! height intervals its triangles occupy (a deck far above the road is one thin slab, an
//! abutment wall or a pole a tall one), and equal neighbours are merged into one box.

use bevy::prelude::*;

/// Meshes whose plan extent is below this stay one box (m).
const SPLIT_MIN_EXTENT_M: f32 = 1.5;
/// The plan column edge is the mesh's plan extent over this many, kept between the limits (m).
const CELLS_ACROSS: f32 = 16.0;
/// Smallest and largest plan column edge (m).
const CELL_RANGE_M: (f32, f32) = (0.3, 2.0);
/// Intervals closer than this in one column are one solid (m).
const MERGE_GAP_M: f32 = 0.4;
/// A surface has at least this thickness (m).
const MIN_THICKNESS_M: f32 = 0.1;
/// More boxes than this and the mesh stays one box.
const MAX_BOXES: usize = 600;
/// Height tolerance for merging neighbouring columns (m).
const SAME_Y_M: f32 = 0.05;

/// The boxes `(lo, hi)` of a triangle mesh in its local axes (x right, y up, z back).
#[must_use]
pub fn boxes_of(positions: &[Vec3], indices: &[u32]) -> Vec<(Vec3, Vec3)> {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for p in positions {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    if lo.x > hi.x {
        return Vec::new();
    }
    let whole = vec![(lo, hi)];
    if (hi.x - lo.x).max(hi.z - lo.z) < SPLIT_MIN_EXTENT_M || indices.len() < 3 {
        return whole;
    }
    let cell =
        ((hi.x - lo.x).max(hi.z - lo.z) / CELLS_ACROSS).clamp(CELL_RANGE_M.0, CELL_RANGE_M.1);
    let cols_x = ((hi.x - lo.x) / cell).ceil().max(1.0) as usize;
    let cols_z = ((hi.z - lo.z) / cell).ceil().max(1.0) as usize;
    if cols_x * cols_z > 40_000 {
        return whole;
    }
    let mut cells: Vec<Vec<(f32, f32)>> = vec![Vec::new(); cols_x * cols_z];
    let mut stack: Vec<[Vec3; 3]> = Vec::new();
    for t in indices.chunks_exact(3) {
        let tri = [0, 1, 2].map(|k| positions.get(t[k] as usize).copied());
        let [Some(a), Some(b), Some(c)] = tri else {
            continue;
        };
        stack.push([a, b, c]);
        while let Some([a, b, c]) = stack.pop() {
            let edge = (a - b)
                .xz()
                .length()
                .max((b - c).xz().length())
                .max((c - a).xz().length());
            if edge > cell * 1.5 && stack.len() < 4096 {
                let (ab, bc, ca) = ((a + b) * 0.5, (b + c) * 0.5, (c + a) * 0.5);
                stack.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
                continue;
            }
            let (tlo, thi) = (a.min(b).min(c), a.max(b).max(c));
            let ix = |x: f32| {
                (((x - lo.x) / cell).floor() as isize).clamp(0, cols_x as isize - 1) as usize
            };
            let iz = |z: f32| {
                (((z - lo.z) / cell).floor() as isize).clamp(0, cols_z as isize - 1) as usize
            };
            for cz in iz(tlo.z)..=iz(thi.z) {
                for cx in ix(tlo.x)..=ix(thi.x) {
                    cells[cz * cols_x + cx].push((tlo.y, thi.y));
                }
            }
        }
    }
    // Per column: the merged height intervals.
    let mut columns: Vec<Vec<(f32, f32)>> = Vec::with_capacity(cells.len());
    for mut spans in cells {
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f32, f32)> = Vec::new();
        for (y0, y1) in spans {
            match merged.last_mut() {
                Some(last) if y0 <= last.1 + MERGE_GAP_M => last.1 = last.1.max(y1),
                _ => merged.push((y0, y1)),
            }
        }
        for m in &mut merged {
            if m.1 - m.0 < MIN_THICKNESS_M {
                m.1 = m.0 + MIN_THICKNESS_M;
            }
        }
        columns.push(merged);
    }
    // Merge equal columns along x into runs, then runs with equal extent along z.
    let mut runs: Vec<(usize, usize, usize, f32, f32)> = Vec::new(); // (z, x0, x1, y0, y1)
    for cz in 0..cols_z {
        for cx in 0..cols_x {
            for &(y0, y1) in &columns[cz * cols_x + cx] {
                let extend = runs.iter_mut().rev().take(8).find(|r| {
                    r.0 == cz
                        && r.2 + 1 == cx
                        && (r.3 - y0).abs() < SAME_Y_M
                        && (r.4 - y1).abs() < SAME_Y_M
                });
                match extend {
                    Some(r) => r.2 = cx,
                    None => runs.push((cz, cx, cx, y0, y1)),
                }
            }
        }
    }
    let mut rects: Vec<(usize, usize, usize, usize, f32, f32)> = Vec::new(); // (z0, z1, x0, x1, y0, y1)
    for (z, x0, x1, y0, y1) in runs {
        let above = rects.iter_mut().find(|r| {
            r.1 + 1 == z
                && r.2 == x0
                && r.3 == x1
                && (r.4 - y0).abs() < SAME_Y_M
                && (r.5 - y1).abs() < SAME_Y_M
        });
        match above {
            Some(r) => r.1 = z,
            None => rects.push((z, z, x0, x1, y0, y1)),
        }
    }
    if rects.len() > MAX_BOXES {
        return whole;
    }
    rects
        .into_iter()
        .map(|(z0, z1, x0, x1, y0, y1)| {
            (
                Vec3::new(lo.x + x0 as f32 * cell, y0, lo.z + z0 as f32 * cell),
                Vec3::new(
                    (lo.x + (x1 + 1) as f32 * cell).min(hi.x),
                    y1,
                    (lo.z + (z1 + 1) as f32 * cell).min(hi.z),
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A quad (two triangles) from corners in order.
    fn quad(v: &mut Vec<Vec3>, i: &mut Vec<u32>, a: Vec3, b: Vec3, c: Vec3, d: Vec3) {
        let n = v.len() as u32;
        v.extend([a, b, c, d]);
        i.extend([n, n + 1, n + 2, n, n + 2, n + 3]);
    }

    /// A bridge over a 10 m wide road: two abutment walls 5 m high, a deck at 5 m, 30 m long.
    fn bridge() -> (Vec<Vec3>, Vec<u32>) {
        let (mut v, mut i) = (Vec::new(), Vec::new());
        // Deck, top and underside, x -20..20, z 0..30.
        for y in [5.0, 5.8] {
            quad(
                &mut v,
                &mut i,
                Vec3::new(-20.0, y, 0.0),
                Vec3::new(20.0, y, 0.0),
                Vec3::new(20.0, y, 30.0),
                Vec3::new(-20.0, y, 30.0),
            );
        }
        // Abutment walls at x = -20..-5 and 5..20 (their inner faces at +-5), z 0..30.
        for (x0, x1) in [(-20.0, -5.0), (5.0, 20.0)] {
            for x in [x0, x1] {
                quad(
                    &mut v,
                    &mut i,
                    Vec3::new(x, 0.0, 0.0),
                    Vec3::new(x, 0.0, 30.0),
                    Vec3::new(x, 5.0, 30.0),
                    Vec3::new(x, 5.0, 0.0),
                );
            }
        }
        (v, i)
    }

    #[test]
    fn a_tiny_mesh_stays_one_box() {
        let v = vec![
            Vec3::ZERO,
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 1.0),
        ];
        assert_eq!(boxes_of(&v, &[0, 1, 2]).len(), 1);
    }

    /// A street lamp drawn in one plane: a 0.2 m pole 7.5 m high and an arm 3.7 m long at 7.2 m.
    /// A bus drives under the arm; the pole stays solid.
    #[test]
    fn a_street_lamp_blocks_its_pole_not_the_space_under_its_arm() {
        let (mut v, mut i) = (Vec::new(), Vec::new());
        quad(
            &mut v,
            &mut i,
            Vec3::new(-0.1, 0.2, 0.0),
            Vec3::new(0.1, 0.2, 0.0),
            Vec3::new(0.1, 7.7, 0.0),
            Vec3::new(-0.1, 7.7, 0.0),
        );
        quad(
            &mut v,
            &mut i,
            Vec3::new(-3.6, 7.2, 0.0),
            Vec3::new(0.1, 7.2, 0.0),
            Vec3::new(0.1, 7.5, 0.0),
            Vec3::new(-3.6, 7.5, 0.0),
        );
        let boxes = boxes_of(&v, &i);
        assert!(boxes.len() > 1, "{boxes:?}");
        let blocks_at = |x: f32, y: f32| {
            boxes
                .iter()
                .any(|(lo, hi)| lo.x <= x && hi.x >= x && lo.y <= y && hi.y >= y)
        };
        assert!(blocks_at(0.0, 2.0), "the pole");
        assert!(!blocks_at(-2.5, 2.0), "under the arm");
        assert!(blocks_at(-2.5, 7.3), "the arm itself");
    }

    #[test]
    fn the_passage_under_a_bridge_deck_stays_free() {
        let (v, i) = bridge();
        let boxes = boxes_of(&v, &i);
        assert!(boxes.len() > 1, "{boxes:?}");
        // A bus (4 m high) at x = 0 in the middle: no box reaches into 0..4.5 m there.
        let blocks = boxes
            .iter()
            .any(|(lo, hi)| lo.x <= 0.5 && hi.x >= -0.5 && lo.y < 4.5 && hi.y > 0.0);
        assert!(!blocks, "{boxes:?}");
        // The abutment walls (faces at x = +-5) still block.
        assert!(boxes
            .iter()
            .any(|(lo, hi)| lo.x <= 5.0 && hi.x >= 5.0 && lo.y <= 0.5 && hi.y >= 4.5));
        // The deck is still there at 5 m over the road.
        assert!(boxes
            .iter()
            .any(|(lo, hi)| lo.x <= 0.0 && hi.x >= 0.0 && lo.y > 4.5 && hi.y > 5.0));
    }
}
