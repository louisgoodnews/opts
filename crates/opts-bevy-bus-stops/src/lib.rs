//! Bus stops of the map (M-0101): the objects whose `.sco` says `[busstop]`.
//!
//! The marker's position and heading are where a bus stops: the heading is the direction of the
//! road, the kerb is on its right. Waiting passengers stand beside it ([`BusStopMarker::waiting_area`]).

use bevy::prelude::*;

/// A bus stop placed in a tile.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BusStopMarker {
    /// The object's ID code in the map.
    pub id_code: u64,
}

/// How far from the marker's line the passengers wait (m, to the right).
pub const WAITING_SIDE_M: f32 = 2.2;
/// Half the length of the waiting area along the road (m).
pub const WAITING_HALF_LENGTH_M: f32 = 3.0;

impl BusStopMarker {
    /// A waiting place in world space for slot `slot` of the stop at `transform`: spread along the
    /// road beside the marker, on the kerb side (the marker's local +X, right of the heading).
    #[must_use]
    pub fn waiting_area(transform: &Transform, slot: u32) -> Vec3 {
        // Bevy: the object's forward is -Z, right is +X.
        let along = (((slot * 7 + 3) % 13) as f32 / 12.0 - 0.5) * 2.0 * WAITING_HALF_LENGTH_M;
        let side = WAITING_SIDE_M + ((slot * 5) % 4) as f32 * 0.35;
        transform.translation + transform.rotation * Vec3::new(side, 0.0, -along)
    }
}

/// The nearest stop within `max_m` of `at` (world space, horizontal distance).
#[must_use]
pub fn nearest_stop<'a>(
    stops: impl IntoIterator<Item = (Entity, &'a Transform)>,
    at: Vec3,
    max_m: f32,
) -> Option<(Entity, f32)> {
    stops
        .into_iter()
        .map(|(e, t)| {
            let d = Vec2::new(t.translation.x - at.x, t.translation.z - at.z).length();
            (e, d)
        })
        .filter(|(_, d)| *d <= max_m)
        .min_by(|a, b| a.1.total_cmp(&b.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passengers_wait_on_the_right_of_the_road_beside_the_stop() {
        // Heading north (-Z in Bevy): right is +X.
        let t = Transform::from_xyz(10.0, 0.0, 20.0);
        for slot in 0..12 {
            let p = BusStopMarker::waiting_area(&t, slot);
            assert!(p.x > 10.0 + 1.9, "right of the road: {p:?}");
            assert!((p.z - 20.0).abs() <= WAITING_HALF_LENGTH_M + 1e-3);
        }
        // Turned half way round the kerb side turns with it.
        let back = Transform::from_xyz(10.0, 0.0, 20.0)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI));
        assert!(BusStopMarker::waiting_area(&back, 0).x < 10.0 - 1.9);
    }

    #[test]
    fn the_nearest_stop_within_range_is_found() {
        let a = Transform::from_xyz(0.0, 0.0, 0.0);
        let b = Transform::from_xyz(30.0, 0.0, 0.0);
        let stops = [
            (Entity::from_raw_u32(1).expect("e"), &a),
            (Entity::from_raw_u32(2).expect("e"), &b),
        ];
        let (e, d) = nearest_stop(stops, Vec3::new(27.0, 0.0, 1.0), 10.0).expect("near");
        assert_eq!(e, Entity::from_raw_u32(2).expect("e"));
        assert!(d < 4.0);
        assert!(nearest_stop(stops, Vec3::new(100.0, 0.0, 0.0), 10.0).is_none());
    }
}
