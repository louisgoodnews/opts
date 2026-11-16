//! The triangle index of the aligned strips and the drawn ground: cell lookup and plane heights.

use std::collections::HashMap;

use bevy::prelude::*;

use super::super::{cross2, GroundTriangle};

/// Triangles indexed by the terrain cells their bounding boxes touch.
#[derive(Debug, Clone, PartialEq, Default)]
pub(in crate::terrain) struct TriIndex {
    /// Counter-clockwise (in east/north) triangles.
    pub(in crate::terrain) tris: Vec<GroundTriangle>,
    cell: f32,
    /// Cell `(i, j)` to the triangles that touch it, in triangle order.
    cells: HashMap<(i32, i32), Vec<u32>>,
}

impl TriIndex {
    /// Indexes `tris` (any winding; degenerate ones are dropped) over cells of `cell` metres.
    pub(super) fn new(tris: Vec<GroundTriangle>, cell: f32) -> Self {
        let tris: Vec<GroundTriangle> = tris
            .into_iter()
            .filter_map(|t| {
                let area = cross2(t[0].truncate(), t[1].truncate(), t[2].truncate());
                if area.abs() < 1e-6 {
                    None
                } else if area < 0.0 {
                    Some([t[0], t[2], t[1]])
                } else {
                    Some(t)
                }
            })
            .collect();
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (n, t) in tris.iter().enumerate() {
            let lo = t[0].min(t[1]).min(t[2]);
            let hi = t[0].max(t[1]).max(t[2]);
            let (i0, j0) = ((lo.x / cell).floor() as i32, (lo.y / cell).floor() as i32);
            let (i1, j1) = ((hi.x / cell).floor() as i32, (hi.y / cell).floor() as i32);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    cells.entry((i, j)).or_default().push(n as u32);
                }
            }
        }
        Self { tris, cell, cells }
    }

    pub(super) fn candidates(&self, i: i32, j: i32) -> &[u32] {
        self.cells.get(&(i, j)).map_or(&[], Vec::as_slice)
    }

    /// The heights at `(x, y)` of the triangles that cover it, in triangle order.
    pub(super) fn heights_at(&self, x: f32, y: f32) -> impl Iterator<Item = f32> + '_ {
        let key = (
            (x / self.cell).floor() as i32,
            (y / self.cell).floor() as i32,
        );
        let p = Vec2::new(x, y);
        self.candidates(key.0, key.1).iter().filter_map(move |&n| {
            let t = &self.tris[n as usize];
            let (a, b, c) = (t[0].truncate(), t[1].truncate(), t[2].truncate());
            let tol = -1e-4;
            (cross2(a, b, p) >= tol && cross2(b, c, p) >= tol && cross2(c, a, p) >= tol)
                .then(|| plane_height(t, p))
        })
    }
}

/// Height of the plane through `t` at `p`.
pub(super) fn plane_height(t: &GroundTriangle, p: Vec2) -> f32 {
    let (a, b, c) = (t[0].truncate(), t[1].truncate(), t[2].truncate());
    let area = cross2(a, b, c);
    if area.abs() < 1e-9 {
        return t[0].z;
    }
    let wa = cross2(b, c, p) / area;
    let wb = cross2(c, a, p) / area;
    t[0].z * wa + t[1].z * wb + t[2].z * (1.0 - wa - wb)
}
