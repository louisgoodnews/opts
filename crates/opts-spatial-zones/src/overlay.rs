//! The spatial-zone debug overlay (M-0115): the box of each resident zone as gizmo lines and a
//! marker at each tracked subject. Off by default; the game switches it with the
//! `spatial_zone_overlay` action. The zones are read from the components the registry spawned, so
//! only resident tiles are drawn. The geometry is pure and lives in `opts-spatial-zones-core`
//! (ISS-092); this module keeps the Bevy resource, the colours and the draw.

use bevy::math::{Isometry3d, Vec3};
use bevy::prelude::*;

use super::model::SpatialZone;
use super::sampler::{ZoneMembership, ZoneSubject};

pub use opts_spatial_zones_core::overlay::{
    ground_corners, SUBJECT_RADIUS_M, ZONE_BOTTOM_M, ZONE_TOP_M,
};

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

/// The twelve edges of the zone box: the floor and the roof rectangles and the four verticals.
#[must_use]
pub fn box_edges(zone: &SpatialZone) -> [(Vec3, Vec3); 12] {
    opts_spatial_zones_core::overlay::box_edges(zone.min, zone.max)
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
