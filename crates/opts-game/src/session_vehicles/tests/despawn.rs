//! Despawning vehicles: the active one, an inactive one, and a trailer chain with its children.

use super::*;

#[test]
fn despawning_an_inactive_vehicle_leaves_the_active_one_alone() {
    let mut app = app();
    let (a, ea) = add(&mut app, "A", "x", Vec2::ZERO);
    let (b, eb) = add(&mut app, "B", "x", Vec2::ZERO);
    ops::switch_to(app.world_mut(), b).expect("b");
    let out = ops::despawn(app.world_mut(), a, DespawnReason::User).expect("despawn a");
    assert_eq!(out.active, Some(b));
    assert!(app.world().get_entity(ea).is_err(), "the entity is gone");
    assert_eq!(players(&mut app), vec![eb]);
    let rec = app
        .world()
        .resource::<SessionVehicles>()
        .get(a)
        .cloned()
        .expect("record");
    assert_eq!(
        (rec.lifecycle, rec.despawn_reason),
        (Lifecycle::Despawned, Some(DespawnReason::User))
    );
    assert!(
        ops::despawn(app.world_mut(), a, DespawnReason::User).is_err(),
        "twice is refused"
    );
}

#[test]
fn despawning_the_active_vehicle_activates_the_most_recent_other_or_none() {
    let mut app = app();
    let (a, _) = add(&mut app, "A", "x", Vec2::ZERO);
    let (b, _) = add(&mut app, "B", "x", Vec2::ZERO);
    let (c, ec) = add(&mut app, "C", "x", Vec2::ZERO);
    ops::switch_to(app.world_mut(), a).expect("a");
    let out = ops::despawn(app.world_mut(), a, DespawnReason::User).expect("despawn a");
    assert_eq!(out.active, Some(c), "the most recently spawned other one");
    assert_eq!(players(&mut app), vec![ec]);
    ops::despawn(app.world_mut(), c, DespawnReason::User).expect("despawn c");
    assert_eq!(
        app.world().resource::<SessionVehicles>().active_id(),
        Some(b)
    );
    let out = ops::despawn(app.world_mut(), b, DespawnReason::User).expect("despawn b");
    assert_eq!(out.active, None);
    assert!(players(&mut app).is_empty());
    assert_eq!(
        app.world().resource::<CameraState>().mode,
        CameraMode::FreeRoam
    );
    assert_eq!(app.world().resource::<WorldStats>().vehicle_name, "");
}

#[test]
fn despawning_removes_the_trailer_chain_and_the_children() {
    let mut app = app();
    let (a, root) = add(&mut app, "A", "x", Vec2::ZERO);
    let seg = app
        .world_mut()
        .spawn((InGameEntity, Transform::default()))
        .id();
    let child = app.world_mut().spawn(InGameEntity).id();
    app.world_mut().entity_mut(root).add_child(child);
    app.world_mut().entity_mut(root).insert(ArticulatedChain {
        segments: vec![seg],
    });
    let out = ops::despawn(app.world_mut(), a, DespawnReason::User).expect("despawn");
    assert_eq!(out.removed, 2, "root and trailer");
    for e in [root, seg, child] {
        assert!(app.world().get_entity(e).is_err(), "{e:?} is gone");
    }
    let leftovers = app
        .world_mut()
        .query_filtered::<Entity, With<InGameEntity>>()
        .iter(app.world())
        .count();
    assert_eq!(leftovers, 0, "nothing of the vehicle is left");
}
