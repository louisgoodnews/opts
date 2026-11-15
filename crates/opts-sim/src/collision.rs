//! Planar collision of oriented boxes: separating-axis test, a spatial grid for many static
//! boxes, and the velocity response.
//!
//! Headings are compass angles in radians (0 = +Y, clockwise): the forward axis of a box is
//! `(sin h, cos h)`, its right axis `(cos h, -sin h)`, as in the vehicle chassis.

use glam::Vec2;

mod grid;

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

pub use grid::ColliderGrid;

#[cfg(test)]
mod tests;
