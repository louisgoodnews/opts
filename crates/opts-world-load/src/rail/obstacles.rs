//! What an AI train must not run into: the footprints of the player's vehicle (every unit of a
//! traction, head to the last unit's rear) and of every unit of the other trains.

use bevy::prelude::*;

use super::super::BodyBox;

/// Footprint of a vehicle used when its `.bus` gives none (a bus: 2.6 m wide, 12 m long).
const DEFAULT_BODY: BodyBox = BodyBox {
    half: Vec2::new(1.3, 6.0),
    offset: Vec2::ZERO,
    height: 3.0,
};

/// One unit's footprint on the ground (bevy x/z plane).
#[derive(Clone, Copy)]
pub(super) struct Obstacle {
    /// The train it belongs to (`Entity::PLACEHOLDER` for the player).
    pub(super) owner: Entity,
    centre: Vec2,
    right: Vec2,
    forward: Vec2,
    half: Vec2,
}

impl Obstacle {
    /// The footprint of a unit at `at` (its rotation gives the axes); `None` body: a bus.
    pub(super) fn new(owner: Entity, at: &Transform, body: Option<BodyBox>) -> Self {
        let body = body.unwrap_or(DEFAULT_BODY);
        let flat = |v: Vec3| Vec2::new(v.x, v.z).normalize_or_zero();
        let (right, forward) = (flat(*at.right()), flat(*at.forward()));
        let pos = Vec2::new(at.translation.x, at.translation.z);
        Self {
            owner,
            centre: pos + right * body.offset.x + forward * body.offset.y,
            right,
            forward,
            half: body.half,
        }
    }

    /// Distance along the ray from `origin` in direction `dir` to where a lane strip of
    /// `margin` half width around it meets the footprint; 0 when the origin is inside.
    fn gap(&self, origin: Vec2, dir: Vec2, margin: f32) -> Option<f32> {
        let rel = origin - self.centre;
        let o = Vec2::new(rel.dot(self.right), rel.dot(self.forward));
        let d = Vec2::new(dir.dot(self.right), dir.dot(self.forward));
        let half = self.half + Vec2::new(margin, 0.0);
        let (mut t0, mut t1) = (0.0_f32, f32::MAX);
        for (o, d, h) in [(o.x, d.x, half.x), (o.y, d.y, half.y)] {
            if d.abs() < 1e-5 {
                if o.abs() > h {
                    return None;
                }
            } else {
                let (a, b) = ((-h - o) / d, (h - o) / d);
                t0 = t0.max(a.min(b));
                t1 = t1.min(a.max(b));
            }
        }
        (t0 <= t1).then_some(t0)
    }
}

/// Distance from the front of a head at `origin` heading `dir` to the nearest obstacle of another
/// owner within `look` metres.
pub(super) fn nearest_gap(
    obstacles: &[Obstacle],
    owner: Entity,
    origin: Vec2,
    dir: Vec2,
    look: f32,
) -> Option<f32> {
    obstacles
        .iter()
        .filter(|o| o.owner != owner)
        .filter_map(|o| o.gap(origin, dir, 0.8))
        .filter(|g| *g <= look)
        .min_by(f32::total_cmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(at: Vec3, yaw: f32) -> Obstacle {
        let tf = Transform::from_translation(at).with_rotation(Quat::from_rotation_y(yaw));
        let body = BodyBox {
            half: Vec2::new(1.2, 10.0),
            offset: Vec2::ZERO,
            height: 3.0,
        };
        Obstacle::new(Entity::PLACEHOLDER, &tf, Some(body))
    }

    #[test]
    fn a_ray_meets_the_near_end_of_a_unit_ahead() {
        // The unit is 20 m long and centred 30 m ahead (bevy forward is -z): its near end is 20 m away.
        let o = unit(Vec3::new(0.0, 0.0, -30.0), 0.0);
        let g = o.gap(Vec2::ZERO, Vec2::new(0.0, -1.0), 0.8).expect("hit");
        assert!((g - 20.0).abs() < 0.01, "{g}");
    }

    #[test]
    fn the_far_unit_of_a_train_is_hit_too_but_a_unit_beside_the_lane_is_not() {
        let beside = unit(Vec3::new(3.5, 0.0, -30.0), 0.0);
        assert!(beside.gap(Vec2::ZERO, Vec2::new(0.0, -1.0), 0.8).is_none());
        let behind = unit(Vec3::new(0.0, 0.0, 30.0), 0.0);
        assert!(behind.gap(Vec2::ZERO, Vec2::new(0.0, -1.0), 0.8).is_none());
    }

    #[test]
    fn inside_the_footprint_the_gap_is_zero() {
        let o = unit(Vec3::ZERO, 0.0);
        assert_eq!(
            o.gap(Vec2::new(0.0, 2.0), Vec2::new(0.0, -1.0), 0.8),
            Some(0.0)
        );
    }
}
