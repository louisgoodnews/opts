//! The spatial-zone debug overlay (M-0115): the box of each resident zone as gizmo lines and a
//! marker at each tracked subject. Off by default; the game switches it with the
//! `spatial_zone_overlay` action. The zones are read from the components the registry spawned, so
//! only resident tiles are drawn.

use bevy::math::{Isometry3d, Vec2, Vec3};
use bevy::prelude::*;

use super::model::SpatialZone;
use super::sampler::{ZoneMembership, ZoneSubject};

/// The floor of a zone box (m above the ground reference).
pub const ZONE_BOTTOM_M: f32 = 0.0;
/// The roof of a zone box drawn for orientation (m); zones have no height in the model.
pub const ZONE_TOP_M: f32 = 4.0;
/// Radius of the subject marker (m).
pub const SUBJECT_RADIUS_M: f32 = 0.6;
const ZONE_COLOUR: Color = Color::srgb(0.9, 0.6, 0.2);
const ZONE_OCCUPIED_COLOUR: Color = Color::srgb(0.3, 1.0, 0.4);
const SUBJECT_OUTSIDE_COLOUR: Color = Color::srgb(0.7, 0.7, 0.7);
const SUBJECT_INSIDE_COLOUR: Color = Color::srgb(0.3, 1.0, 0.4);

/// Whether the zone overlay is drawn. The toggle is the only writer.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ZoneOverlay {
    on: bool,
}

impl ZoneOverlay {
    /// Whether the overlay is drawn.
    #[must_use]
    pub fn on(&self) -> bool {
        self.on
    }

    /// Switches the overlay on or off.
    pub fn toggle(&mut self) {
        self.on = !self.on;
    }
}

/// The four ground corners of the box `min`..`max` at height `y`, in the order south-west,
/// south-east, north-east, north-west. The ground point of a zone is `(x, z)` of the Bevy
/// transform, the same point the sampler tests, so a corner `c` satisfies
/// `zone.contains(Vec2::new(c.x, c.z))` for the zone that owns it.
#[must_use]
pub fn ground_corners(min: Vec2, max: Vec2, y: f32) -> [Vec3; 4] {
    [
        Vec3::new(min.x, y, min.y),
        Vec3::new(max.x, y, min.y),
        Vec3::new(max.x, y, max.y),
        Vec3::new(min.x, y, max.y),
    ]
}

/// The twelve edges of the zone box: the floor and the roof rectangles and the four verticals.
#[must_use]
pub fn box_edges(zone: &SpatialZone) -> [(Vec3, Vec3); 12] {
    let floor = ground_corners(zone.min, zone.max, ZONE_BOTTOM_M);
    let roof = ground_corners(zone.min, zone.max, ZONE_TOP_M);
    let mut edges = [(Vec3::ZERO, Vec3::ZERO); 12];
    for i in 0..4 {
        let next = (i + 1) % 4;
        edges[i] = (floor[i], floor[next]);
        edges[4 + i] = (roof[i], roof[next]);
        edges[8 + i] = (floor[i], roof[i]);
    }
    edges
}

/// Draws every resident zone (green when a subject is inside it) and a marker per subject.
pub fn draw_zone_overlay(
    overlay: Res<ZoneOverlay>,
    zones: Query<&SpatialZone>,
    subjects: Query<(&Transform, &ZoneMembership), With<ZoneSubject>>,
    mut gizmos: Gizmos,
) {
    if !overlay.on() {
        return;
    }
    for zone in &zones {
        let occupied = subjects
            .iter()
            .any(|(_, m)| m.zone.is_some_and(|(id, _)| id == zone.id));
        let colour = if occupied {
            ZONE_OCCUPIED_COLOUR
        } else {
            ZONE_COLOUR
        };
        for (a, b) in box_edges(zone) {
            gizmos.line(a, b, colour);
        }
    }
    for (transform, membership) in &subjects {
        let colour = if membership.tracker.inside() {
            SUBJECT_INSIDE_COLOUR
        } else {
            SUBJECT_OUTSIDE_COLOUR
        };
        gizmos.sphere(
            Isometry3d::from_translation(transform.translation),
            SUBJECT_RADIUS_M,
            colour,
        );
    }
}

#[cfg(test)]
#[path = "overlay_tests.rs"]
mod tests;
