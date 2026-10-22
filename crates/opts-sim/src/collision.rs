//! Planar collision of oriented boxes: separating-axis test, a spatial grid for many static
//! boxes, and the velocity response.
//!
//! Headings are compass angles in radians (0 = +Y, clockwise): the forward axis of a box is
//! `(sin h, cos h)`, its right axis `(cos h, -sin h)`, as in the vehicle chassis.

use std::collections::HashMap;

use glam::Vec2;

/// An oriented box on the ground plane with a height range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb {
    /// Centre.
    pub centre: Vec2,
    /// Half extents `(right, forward)`.
    pub half: Vec2,
    /// Compass heading (rad).
    pub heading: f32,
    /// Lowest point (m).
    pub z_min: f32,
    /// Highest point (m).
    pub z_max: f32,
}

/// The overlap of two boxes: the shortest push that separates them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// Unit vector pointing from the other box towards this one.
    pub normal: Vec2,
    /// Penetration depth along the normal (m).
    pub depth: f32,
}

impl Obb {
    /// A box.
    #[must_use]
    pub fn new(centre: Vec2, half: Vec2, heading: f32, z_min: f32, z_max: f32) -> Self {
        Self {
            centre,
            half,
            heading,
            z_min,
            z_max,
        }
    }

    /// Forward and right unit axes.
    #[must_use]
    pub fn axes(&self) -> [Vec2; 2] {
        let (s, c) = self.heading.sin_cos();
        [Vec2::new(s, c), Vec2::new(c, -s)]
    }

    /// Whether the plan point `p` lies inside the box (its height is not looked at).
    #[must_use]
    pub fn contains(&self, p: Vec2) -> bool {
        let [fwd, right] = self.axes();
        let d = p - self.centre;
        d.dot(right).abs() <= self.half.x && d.dot(fwd).abs() <= self.half.y
    }

    /// Radius of the circle around the centre that contains the box.
    #[must_use]
    pub fn radius(&self) -> f32 {
        self.half.length()
    }

    /// The four corners.
    #[must_use]
    pub fn corners(&self) -> [Vec2; 4] {
        let [f, r] = self.axes();
        let (hr, hf) = (r * self.half.x, f * self.half.y);
        [
            self.centre + hr + hf,
            self.centre - hr + hf,
            self.centre - hr - hf,
            self.centre + hr - hf,
        ]
    }

    /// Half the extent of this box along a unit axis.
    fn support(&self, axis: Vec2) -> f32 {
        let [f, r] = self.axes();
        self.half.x * r.dot(axis).abs() + self.half.y * f.dot(axis).abs()
    }

    /// Whether the height ranges overlap by more than `slack` (a box whose top is within `slack`
    /// of the other's bottom can be driven over).
    #[must_use]
    pub fn overlaps_height(&self, other: &Obb, slack: f32) -> bool {
        self.z_min < other.z_max - slack && other.z_min < self.z_max
    }

    /// The contact of `self` with `other` (separating-axis test on the four edge normals), or
    /// `None` if they do not overlap.
    #[must_use]
    pub fn contact(&self, other: &Obb) -> Option<Contact> {
        let d = self.centre - other.centre;
        let mut best: Option<Contact> = None;
        for axis in self.axes().into_iter().chain(other.axes()) {
            let overlap = self.support(axis) + other.support(axis) - d.dot(axis).abs();
            if overlap <= 0.0 {
                return None;
            }
            if best.is_none_or(|b| overlap < b.depth) {
                let sign = if d.dot(axis) >= 0.0 { 1.0 } else { -1.0 };
                best = Some(Contact {
                    normal: axis * sign,
                    depth: overlap,
                });
            }
        }
        best
    }
}

/// Removes the velocity component into the obstacle: `normal` points from the obstacle to the
/// mover, `restitution` is the bounce (0 = dead stop against the wall, 1 = elastic), `friction`
/// the share of the tangential speed lost.
#[must_use]
pub fn respond(velocity: Vec2, normal: Vec2, restitution: f32, friction: f32) -> Vec2 {
    let vn = velocity.dot(normal);
    if vn >= 0.0 {
        return velocity; // already moving away
    }
    let tangent = velocity - normal * vn;
    normal * (-vn * restitution) + tangent * (1.0 - friction)
}

/// Static boxes in a uniform grid; entries carry a key (the tile) so a tile's boxes can be
/// dropped together.
#[derive(Debug, Clone)]
pub struct ColliderGrid<K> {
    cell: f32,
    cells: HashMap<(i32, i32), Vec<(K, Obb)>>,
}

impl<K: Clone + PartialEq> ColliderGrid<K> {
    /// A grid with square cells of `cell` metres.
    #[must_use]
    pub fn new(cell: f32) -> Self {
        Self {
            cell,
            cells: HashMap::new(),
        }
    }

    fn index(&self, p: Vec2) -> (i32, i32) {
        (
            (p.x / self.cell).floor() as i32,
            (p.y / self.cell).floor() as i32,
        )
    }

    /// Adds a box to every cell its bounding circle touches.
    pub fn insert(&mut self, key: K, obb: Obb) {
        let r = obb.radius();
        let (lo, hi) = (
            self.index(obb.centre - Vec2::splat(r)),
            self.index(obb.centre + Vec2::splat(r)),
        );
        for x in lo.0..=hi.0 {
            for y in lo.1..=hi.1 {
                self.cells
                    .entry((x, y))
                    .or_default()
                    .push((key.clone(), obb));
            }
        }
    }

    /// Drops every box inserted under `key`.
    pub fn remove(&mut self, key: &K) {
        self.cells.retain(|_, v| {
            v.retain(|(k, _)| k != key);
            !v.is_empty()
        });
    }

    /// The boxes in the cells within `radius` of `centre` (a box can appear in several cells).
    pub fn query(&self, centre: Vec2, radius: f32) -> impl Iterator<Item = &Obb> {
        let (lo, hi) = (
            self.index(centre - Vec2::splat(radius)),
            self.index(centre + Vec2::splat(radius)),
        );
        (lo.0..=hi.0)
            .flat_map(move |x| (lo.1..=hi.1).map(move |y| (x, y)))
            .filter_map(|c| self.cells.get(&c))
            .flatten()
            .map(|(_, o)| o)
    }

    /// Number of stored (box, cell) entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.values().map(Vec::len).sum()
    }

    /// Whether the grid is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bx(x: f32, y: f32, hr: f32, hf: f32, h: f32) -> Obb {
        Obb::new(Vec2::new(x, y), Vec2::new(hr, hf), h, 0.0, 2.0)
    }

    #[test]
    fn separated_boxes_do_not_touch() {
        assert!(bx(0.0, 0.0, 1.0, 2.0, 0.0)
            .contact(&bx(5.0, 0.0, 1.0, 1.0, 0.0))
            .is_none());
    }

    #[test]
    fn overlap_gives_the_shortest_push_away_from_the_other() {
        // `a` 2 m wide and 4 m long heading north; `b` 1 m to its right and overlapping by 1 m.
        let a = bx(0.0, 0.0, 1.0, 2.0, 0.0);
        let b = bx(1.5, 0.0, 1.0, 1.0, 0.0);
        let c = a.contact(&b).expect("overlap");
        assert!((c.depth - 0.5).abs() < 1e-4, "{c:?}");
        assert!(c.normal.x < -0.99, "pushed to the left: {c:?}");
    }

    #[test]
    fn rotated_boxes_use_their_own_axes() {
        // Turned by 90° the long side lies along x and reaches x = 2; the box at 2.25 starts at 1.75.
        let a = bx(0.0, 0.0, 1.0, 2.0, std::f32::consts::FRAC_PI_2);
        let b = bx(2.25, 0.0, 0.5, 0.5, 0.0);
        let c = a.contact(&b).expect("overlap");
        assert!((c.depth - 0.25).abs() < 1e-3, "{c:?}");
        assert!(bx(0.0, 0.0, 1.0, 2.0, std::f32::consts::FRAC_PI_2)
            .contact(&bx(3.0, 0.0, 0.5, 0.5, 0.0))
            .is_none());
    }

    #[test]
    fn response_stops_the_normal_component_and_keeps_part_of_the_tangent() {
        let v = respond(Vec2::new(-3.0, 4.0), Vec2::new(1.0, 0.0), 0.0, 0.25);
        assert!(v.x.abs() < 1e-6);
        assert!((v.y - 3.0).abs() < 1e-6);
        // Moving away is left alone.
        assert_eq!(
            respond(Vec2::new(2.0, 1.0), Vec2::new(1.0, 0.0), 0.0, 0.5),
            Vec2::new(2.0, 1.0)
        );
    }

    #[test]
    fn height_slack_lets_a_vehicle_drive_over_a_kerb() {
        let car = Obb::new(Vec2::ZERO, Vec2::ONE, 0.0, 0.0, 1.5);
        let kerb = Obb::new(Vec2::ZERO, Vec2::ONE, 0.0, 0.0, 0.15);
        assert!(!car.overlaps_height(&kerb, 0.3));
        let wall = Obb::new(Vec2::ZERO, Vec2::ONE, 0.0, 0.0, 1.0);
        assert!(car.overlaps_height(&wall, 0.3));
    }

    #[test]
    fn grid_finds_nearby_boxes_and_drops_a_tile() {
        let mut g = ColliderGrid::new(10.0);
        g.insert(1, bx(5.0, 5.0, 1.0, 1.0, 0.0));
        g.insert(2, bx(95.0, 5.0, 1.0, 1.0, 0.0));
        assert_eq!(g.query(Vec2::new(5.0, 5.0), 3.0).count(), 1);
        g.remove(&1);
        assert_eq!(g.query(Vec2::new(5.0, 5.0), 3.0).count(), 0);
        assert!(!g.is_empty());
    }
}
