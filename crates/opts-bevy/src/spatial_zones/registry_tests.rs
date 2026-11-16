//! Tests of the per-tile zone registry: build and drop, duplicate loads and the entity-count
//! stability over load/unload cycles.

use bevy::math::Vec2;
use bevy::prelude::*;

use super::super::model::{SpatialZone, ZoneId, ZoneKind};
use super::{TileKey, ZoneRegistry};

fn zone(id: u32) -> SpatialZone {
    SpatialZone {
        id: ZoneId(id),
        kind: ZoneKind::Tunnel,
        min: Vec2::ZERO,
        max: Vec2::new(40.0, 8.0),
    }
}

/// Runs `f` with commands, flushes them and returns its result.
fn with_commands<R>(app: &mut App, f: impl FnOnce(&mut Commands, &mut ZoneRegistry) -> R) -> R {
    let world = app.world_mut();
    let mut registry = world.remove_resource::<ZoneRegistry>().unwrap_or_default();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let result = {
        let mut commands = Commands::new(&mut queue, world);
        f(&mut commands, &mut registry)
    };
    queue.apply(world);
    world.insert_resource(registry);
    result
}

fn zone_entities(app: &mut App) -> usize {
    app.world_mut()
        .query::<&SpatialZone>()
        .iter(app.world())
        .count()
}

#[test]
fn build_spawns_one_entity_per_zone_and_drop_removes_them() {
    let mut app = App::new();
    let tile: TileKey = (3, -7);
    let spawned = with_commands(&mut app, |c, r| r.build_tile(c, tile, &[zone(1), zone(2)]));
    assert_eq!(spawned, 2);
    assert_eq!(zone_entities(&mut app), 2);
    let removed = with_commands(&mut app, |c, r| r.drop_tile(c, tile));
    assert_eq!(removed, 2);
    assert_eq!(zone_entities(&mut app), 0);
}

#[test]
fn rebuilding_a_tile_replaces_its_zones() {
    let mut app = App::new();
    let tile: TileKey = (0, 0);
    with_commands(&mut app, |c, r| r.build_tile(c, tile, &[zone(1)]));
    with_commands(&mut app, |c, r| r.build_tile(c, tile, &[zone(2), zone(3)]));
    assert_eq!(zone_entities(&mut app), 2);
    let registry_entities = with_commands(&mut app, |_, r| r.entity_count());
    assert_eq!(registry_entities, 2);
}

#[test]
fn dropping_an_unknown_tile_is_a_no_op() {
    let mut app = App::new();
    let removed = with_commands(&mut app, |c, r| r.drop_tile(c, (9, 9)));
    assert_eq!(removed, 0);
}

#[test]
fn load_unload_cycles_leave_the_entity_count_stable() {
    let mut app = App::new();
    // The first access inserts the registry resource, which is one entity; take the baseline after it.
    with_commands(&mut app, |_, r| r.tile_count());
    let baseline = app.world_mut().iter_entities().count();
    for cycle in 0..25 {
        let tile: TileKey = (cycle % 4, 0);
        with_commands(&mut app, |c, r| r.build_tile(c, tile, &[zone(1), zone(2)]));
        with_commands(&mut app, |c, r| r.drop_tile(c, tile));
    }
    assert_eq!(zone_entities(&mut app), 0);
    let with_registry = with_commands(&mut app, |_, r| r.tile_count());
    assert_eq!(with_registry, 0);
    assert_eq!(app.world_mut().iter_entities().count(), baseline);
}

#[test]
fn reconcile_builds_resident_tiles_and_drops_unloaded_ones() {
    let mut app = App::new();
    with_commands(&mut app, |_, r| r.tile_count());
    let baseline = app.world_mut().iter_entities().count();
    let zones_of = |_: TileKey| vec![zone(1), zone(2)];
    let a: TileKey = (0, 0);
    let b: TileKey = (1, 0);
    let built = with_commands(&mut app, |c, r| r.reconcile(c, &[a, b], zones_of));
    assert_eq!(built, (2, 0));
    assert_eq!(zone_entities(&mut app), 4);
    let dropped = with_commands(&mut app, |c, r| r.reconcile(c, &[a], zones_of));
    assert_eq!(dropped, (0, 1));
    assert_eq!(zone_entities(&mut app), 2);
    let tiles = with_commands(&mut app, |_, r| (r.tile_count(), r.contains(b)));
    assert_eq!(tiles, (1, false));
    with_commands(&mut app, |c, r| r.reconcile(c, &[], zones_of));
    assert_eq!(zone_entities(&mut app), 0);
    assert_eq!(app.world_mut().iter_entities().count(), baseline);
}

#[test]
fn reconcile_with_unchanged_residents_is_a_no_op() {
    let mut app = App::new();
    let zones_of = |_: TileKey| vec![zone(1)];
    with_commands(&mut app, |c, r| r.reconcile(c, &[(5, 5)], zones_of));
    let again = with_commands(&mut app, |c, r| r.reconcile(c, &[(5, 5)], zones_of));
    assert_eq!(again, (0, 0));
    assert_eq!(zone_entities(&mut app), 1);
}
