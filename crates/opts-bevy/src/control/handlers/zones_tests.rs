//! Tests of the `zones.query` handler: the payload through the world, the subject and the
//! `limit` parameter.

use bevy::math::Vec2;
use bevy::prelude::*;
use serde_json::json;

use super::zones_query;
use crate::control::state::Outcome;
use crate::spatial_zones::sampler::step;
use crate::spatial_zones::{SpatialZone, ZoneId, ZoneKind, ZoneMembership, ZoneSubject};
use opts_cli::protocol::code;

fn tunnel(id: u32) -> SpatialZone {
    SpatialZone {
        id: ZoneId(id),
        kind: ZoneKind::Tunnel,
        min: Vec2::new(0.0, 0.0),
        max: Vec2::new(100.0, 8.0),
    }
}

/// Runs the handler and unwraps its immediate answer.
fn answer(world: &mut World, params: &serde_json::Value) -> Result<serde_json::Value, i64> {
    match zones_query(world, params) {
        Outcome::Done(Ok(v)) => Ok(v),
        Outcome::Done(Err(e)) => Err(e.code),
        Outcome::Wait(_) => panic!("zones.query answers at once"),
    }
}

#[test]
fn handler_lists_the_zones_of_the_world_without_a_subject() {
    let mut world = World::new();
    world.spawn(tunnel(1));
    world.spawn(tunnel(2));
    let v = answer(&mut world, &json!({})).expect("answer");
    assert_eq!(v["count"], 2);
    assert_eq!(v["zones"].as_array().map(Vec::len), Some(2));
    assert!(v["subject"].is_null());
}

#[test]
fn handler_reports_the_subject_when_it_has_a_sample() {
    let mut world = World::new();
    let zones = [tunnel(7)];
    world.spawn(zones[0]);
    let mut m = ZoneMembership::new(2.0, 2.0);
    let p = Vec2::new(50.0, 4.0);
    step(&mut m, p, &zones);
    step(&mut m, p, &zones);
    world.spawn((ZoneSubject, m));
    let v = answer(&mut world, &json!({})).expect("answer");
    assert_eq!(v["subject"]["inside"], true);
    assert_eq!(v["subject"]["zone"]["id"], 7);
    assert_eq!(v["subject"]["point"][1], 4.0);
}

#[test]
fn subject_without_a_sample_is_null() {
    let mut world = World::new();
    world.spawn((ZoneSubject, ZoneMembership::default()));
    let v = answer(&mut world, &json!({})).expect("answer");
    assert!(v["subject"].is_null());
}

#[test]
fn limit_caps_the_listing_and_bad_limits_are_refused() {
    let mut world = World::new();
    for id in 0..3 {
        world.spawn(tunnel(id));
    }
    let v = answer(&mut world, &json!({"limit": 1})).expect("answer");
    assert_eq!(v["count"], 3);
    assert_eq!(v["zones"].as_array().map(Vec::len), Some(1));
    let bad = answer(&mut world, &json!({"limit": "many"}));
    assert_eq!(bad, Err(code::INVALID_PARAMS));
}
