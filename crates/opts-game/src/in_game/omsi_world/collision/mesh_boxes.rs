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

/// The height span of every triangle (cut into pieces no longer than the cell) in each cell of the
/// grid over the footprint.
fn fill_cells(
    positions: &[Vec3],
    indices: &[u32],
    (lo, cell): (Vec3, f32),
    (cols_x, cols_z): (usize, usize),
) -> Vec<Vec<(f32, f32)>> {
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
    cells
}

/// Per column: the merged height intervals, none thinner than the minimum thickness.
fn merge_columns(cells: Vec<Vec<(f32, f32)>>) -> Vec<Vec<(f32, f32)>> {
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
    columns
}

/// A rectangle of cells with one height span: (z0, z1, x0, x1, y0, y1).
type Rect = (usize, usize, usize, usize, f32, f32);

/// Merges equal columns along x into runs, then runs with equal extent along z.
fn merge_rects(columns: &[Vec<(f32, f32)>], (cols_x, cols_z): (usize, usize)) -> Vec<Rect> {
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
    let mut rects: Vec<Rect> = Vec::new();
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
    rects
}

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
    let cells = fill_cells(positions, indices, (lo, cell), (cols_x, cols_z));
    let columns = merge_columns(cells);
    let rects = merge_rects(&columns, (cols_x, cols_z));
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
mod tests;
