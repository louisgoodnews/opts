//! Lanes as gizmo lines: the centre line of each lane near the camera and an arrow from each
//! lane's end to the start of each successor.

use bevy::math::DVec2;
use bevy::prelude::*;
use opts_bevy::MainCamera;
use opts_traffic::{Lane, LaneKind};

use super::super::TrafficState;
use super::LaneOverlay;
use crate::WorldStats;

/// Lanes with neither end within this distance of the camera (m) are not drawn.
pub const LANE_RADIUS_M: f64 = 250.0;
/// Spacing of the centre-line samples (m).
const SAMPLE_STEP_M: f64 = 10.0;
/// Successor arrows.
const ARROW_COLOUR: Color = Color::srgb(1.0, 0.9, 0.3);

/// A session-frame point (east, north) at `height` as a Bevy position (x east, y up, z = -north).
#[must_use]
pub fn bevy_point(p: DVec2, height: f64) -> Vec3 {
    Vec3::new(p.x as f32, height as f32, -p.y as f32)
}

/// The session-frame position (east, north) of a Bevy transform.
#[must_use]
pub fn session_xy(t: &GlobalTransform) -> DVec2 {
    let v = t.translation();
    DVec2::new(f64::from(v.x), -f64::from(v.z))
}

/// The centre line of `lane` as Bevy points: a sample every `step` metres, both ends included.
#[must_use]
pub fn lane_samples(lane: &Lane, step: f64) -> Vec<Vec3> {
    let len = lane.length();
    let n = (len / step).ceil().max(1.0) as usize;
    (0..=n)
        .map(|i| {
            let s = len * i as f64 / n as f64;
            bevy_point(lane.geometry.point_at(s), lane.z_at(s))
        })
        .collect()
}

/// The arrow from the end of `from` to the start of its successor `to`, as Bevy points.
#[must_use]
pub fn successor_arrow(from: &Lane, to: &Lane) -> (Vec3, Vec3) {
    let end = from.length();
    (
        bevy_point(from.geometry.end(), from.z_at(end)),
        bevy_point(to.geometry.start, to.z_at(0.0)),
    )
}

/// Whether either end of `lane` lies within [`LANE_RADIUS_M`] of `centre`.
#[must_use]
pub fn lane_near(lane: &Lane, centre: DVec2) -> bool {
    lane.geometry.start.distance(centre) <= LANE_RADIUS_M
        || lane.geometry.end().distance(centre) <= LANE_RADIUS_M
}

/// The colour of a lane's centre line by the traffic it carries.
fn lane_colour(kind: LaneKind) -> Color {
    match kind {
        LaneKind::Road => Color::srgb(0.3, 0.85, 1.0),
        LaneKind::Pedestrian => Color::srgb(0.6, 1.0, 0.4),
        LaneKind::Rail => Color::srgb(1.0, 0.6, 0.3),
    }
}

/// Draws the centre lines near the camera and the successor arrows between them.
pub fn draw_lanes(
    stats: Res<WorldStats>,
    overlay: Res<LaneOverlay>,
    traffic: Option<Res<TrafficState>>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
    mut gizmos: Gizmos,
) {
    if !(stats.overlay_visible && overlay.lanes) {
        return;
    }
    let (Some(traffic), Ok(cam)) = (traffic, camera.single()) else {
        return;
    };
    let centre = session_xy(cam);
    let graph = traffic.lane_graph();
    for id in graph.lane_ids() {
        let Some(lane) = graph.lane(id) else {
            continue;
        };
        if !lane_near(lane, centre) {
            continue;
        }
        gizmos.linestrip(lane_samples(lane, SAMPLE_STEP_M), lane_colour(lane.kind));
        for &next in graph.successors(id) {
            if let Some(to) = graph.lane(next) {
                let (from_p, to_p) = successor_arrow(lane, to);
                gizmos.arrow(from_p, to_p, ARROW_COLOUR);
            }
        }
    }
}
