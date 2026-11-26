//! Tests of the membership step: one transition per crossing, no flapping, the zone id carried
//! to the leave message and the sampling gate.

use bevy::math::Vec2;

use opts_spatial_zones::model::{SpatialZone, ZoneId, ZoneKind};
use opts_spatial_zones::sampler::{step, Emitted, ZoneMembership};

fn tunnel(id: u32, min_x: f32, max_x: f32) -> SpatialZone {
    SpatialZone {
        id: ZoneId(id),
        kind: ZoneKind::Tunnel,
        min: Vec2::new(min_x, 0.0),
        max: Vec2::new(max_x, 8.0),
    }
}

/// Drives a subject along x from `from` to `to` in `step_m` increments at y = 4, collecting the
/// emitted transitions.
fn drive(
    m: &mut ZoneMembership,
    zones: &[SpatialZone],
    from: f32,
    to: f32,
    step_m: f32,
) -> Vec<Emitted> {
    let mut x = from;
    let mut out = Vec::new();
    while (to - x).abs() > 1e-4 {
        x += step_m * (to - from).signum();
        out.extend(step(m, Vec2::new(x, 4.0), zones));
    }
    out
}

#[test]
fn a_drive_through_a_tunnel_emits_one_enter_and_one_leave() {
    let zones = [tunnel(7, 100.0, 300.0)];
    let mut m = ZoneMembership::new(15.0, 5.0);
    let events = drive(&mut m, &zones, 0.0, 400.0, 2.0);
    assert_eq!(
        events,
        vec![
            Emitted::Entered(ZoneId(7), ZoneKind::Tunnel),
            Emitted::Left(ZoneId(7), ZoneKind::Tunnel),
        ]
    );
    assert!(!m.tracker.inside());
    assert_eq!(m.zone, None);
}

#[test]
fn a_short_pass_under_a_canopy_emits_nothing() {
    let zones = [tunnel(1, 100.0, 110.0)];
    let mut m = ZoneMembership::new(15.0, 5.0);
    assert!(drive(&mut m, &zones, 0.0, 200.0, 1.0).is_empty());
}

#[test]
fn a_gap_inside_the_zone_does_not_end_it() {
    let zones = [tunnel(2, 0.0, 100.0), tunnel(3, 102.0, 200.0)];
    let mut m = ZoneMembership::new(15.0, 5.0);
    let events = drive(&mut m, &zones, -20.0, 220.0, 1.0);
    assert_eq!(
        events,
        vec![
            Emitted::Entered(ZoneId(2), ZoneKind::Tunnel),
            Emitted::Left(ZoneId(2), ZoneKind::Tunnel),
        ],
        "a 2 m gap between the boxes is below the 5 m leave distance"
    );
}

#[test]
fn a_subject_that_stands_still_keeps_its_state() {
    let zones = [tunnel(4, 0.0, 100.0)];
    let mut m = ZoneMembership::new(15.0, 5.0);
    let p = Vec2::new(50.0, 4.0);
    let events: Vec<Emitted> = (0..50).filter_map(|_| step(&mut m, p, &zones)).collect();
    assert!(events.is_empty(), "no distance travelled, no transition");
}
