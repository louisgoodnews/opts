//! The exhaust outlet of a rail consist (ISS-038): the units behind the head are the consist's
//! followers, and the rear-most one with an emitter gives the outlet, as an articulated chain does.

use std::collections::HashMap;

use bevy::ecs::world::World;
use bevy::prelude::*;
use opts_bevy::vehicle::ExhaustOutlet;
use opts_game::in_game::omsi_world::{Part, VehicleComposition};
use opts_game::in_game::weather::behind;
use opts_game::in_game::weather::exhaust::{chain_outlet, fallback_outlet_of, outlet_of};

type Units = HashMap<Entity, (GlobalTransform, Option<ExhaustOutlet>)>;

fn vehicle() -> GlobalTransform {
    GlobalTransform::from(Transform::from_xyz(10.0, 0.0, 0.0))
}

/// A follower at `z` behind the head, with its own emitter (or none).
fn follower(world: &mut World, units: &mut Units, z: f32, emitter: Option<Vec3>) -> Part {
    let entity = world.spawn_empty().id();
    let at = GlobalTransform::from(Transform::from_xyz(10.0, 0.0, z));
    units.insert(entity, (at, emitter.map(ExhaustOutlet)));
    Part {
        entity,
        name: format!("car{z}"),
        behind: -z,
        half_span: 0.0,
        reversed: false,
        meshes: 0,
        vertices: 0,
        body: None,
    }
}

/// The outlet of a rail head with `followers` behind it and `head` as its own emitter.
fn rail_outlet(followers: Vec<Part>, head: Option<&ExhaustOutlet>, units: &Units) -> Vec3 {
    let consist = VehicleComposition {
        head: Part {
            entity: Entity::PLACEHOLDER,
            name: "head".into(),
            behind: 0.0,
            half_span: 0.0,
            reversed: false,
            meshes: 0,
            vertices: 0,
            body: None,
        },
        followers,
    };
    let ids = behind(None, Some(&consist));
    let units = ids
        .iter()
        .filter_map(|e| units.get(e))
        .map(|(gt, emitter)| (gt, emitter.as_ref()));
    chain_outlet(&vehicle(), head, units)
}

#[test]
fn a_rail_consist_with_an_emitter_on_its_rear_unit_gives_that_outlet() {
    let mut world = World::new();
    let mut units = Units::new();
    let front = follower(&mut world, &mut units, -4.0, None);
    let rear = follower(&mut world, &mut units, -12.0, Some(Vec3::Y * 2.0));
    let outlet = rail_outlet(vec![front, rear], None, &units);
    assert_eq!(outlet, Vec3::new(10.0, 2.0, -12.0));
    assert_ne!(outlet, fallback_outlet_of(&vehicle()));
}

#[test]
fn the_rear_most_emitter_wins_over_a_front_one() {
    let mut world = World::new();
    let mut units = Units::new();
    let front = follower(&mut world, &mut units, -4.0, Some(Vec3::Y * 3.0));
    let rear = follower(&mut world, &mut units, -12.0, Some(Vec3::Y * 2.0));
    let outlet = rail_outlet(vec![front, rear], None, &units);
    assert_eq!(outlet, Vec3::new(10.0, 2.0, -12.0));
}

#[test]
fn a_rail_consist_without_an_emitter_gives_the_fallback() {
    let mut world = World::new();
    let mut units = Units::new();
    let front = follower(&mut world, &mut units, -4.0, None);
    let rear = follower(&mut world, &mut units, -12.0, None);
    let outlet = rail_outlet(vec![front, rear], None, &units);
    assert_eq!(outlet, fallback_outlet_of(&vehicle()));
}

#[test]
fn the_head_outlet_is_the_last_choice_for_a_rail_consist() {
    let mut world = World::new();
    let mut units = Units::new();
    let rear = follower(&mut world, &mut units, -12.0, None);
    let head = ExhaustOutlet(Vec3::new(0.0, 1.0, 2.0));
    let outlet = rail_outlet(vec![rear], Some(&head), &units);
    assert_eq!(outlet, outlet_of(&vehicle(), Some(&head)));
    assert_ne!(outlet, fallback_outlet_of(&vehicle()));
}
