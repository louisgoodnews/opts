//! Tests of the `zones.query` payload.

use bevy::math::Vec2;

use opts_spatial_zones::model::{SpatialZone, ZoneId, ZoneKind};
use opts_spatial_zones::query::zones_payload;
use opts_spatial_zones::sampler::{step, ZoneMembership};

fn tunnel(id: u32) -> SpatialZone {
    SpatialZone {
        id: ZoneId(id),
        kind: ZoneKind::Tunnel,
        min: Vec2::new(0.0, 0.0),
        max: Vec2::new(100.0, 8.0),
    }
}

#[test]
fn payload_without_a_subject_lists_the_zones() {
    let v = zones_payload(&[tunnel(1), tunnel(2)], None, 10);
    assert_eq!(v["count"], 2);
    assert_eq!(v["zones"][1]["id"], 2);
    assert_eq!(v["zones"][0]["kind"], "tunnel");
    assert!(v["subject"].is_null());
}

#[test]
fn payload_reports_the_membership_of_the_subject() {
    let zones = [tunnel(5)];
    let mut m = ZoneMembership::new(2.0, 2.0);
    let p = Vec2::new(50.0, 4.0);
    step(&mut m, p, &zones);
    step(&mut m, p, &zones);
    step(&mut m, Vec2::new(60.0, 4.0), &zones);
    let v = zones_payload(&zones, Some((p, &m)), 10);
    assert_eq!(v["subject"]["inside"], true);
    assert_eq!(v["subject"]["zone"]["id"], 5);
    assert_eq!(v["subject"]["point"][0], 50.0);
}

#[test]
fn limit_caps_the_listed_zones_but_not_the_count() {
    let zones = [tunnel(1), tunnel(2), tunnel(3)];
    let v = zones_payload(&zones, None, 2);
    assert_eq!(v["count"], 3);
    assert_eq!(v["zones"].as_array().map(Vec::len), Some(2));
}
