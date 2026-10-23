//! Live mirrors on the vehicles around the player.

use super::{Scene, *};

fn ambient_cameras(s: &mut Scene) -> Vec<Entity> {
    let mut q = s
        .app
        .world_mut()
        .query_filtered::<Entity, With<AmbientCamera>>();
    q.iter(s.app.world()).collect()
}

fn material_of(s: &Scene, pane: Entity) -> Handle<StandardMaterial> {
    s.app
        .world()
        .get::<MeshMaterial3d<StandardMaterial>>(pane)
        .unwrap()
        .0
        .clone()
}

fn run(s: &mut Scene, frames: usize) {
    for _ in 0..frames {
        s.app.update();
    }
}

#[test]
fn the_nearest_panes_of_another_vehicle_get_live_pictures_and_the_player_vehicle_does_not() {
    // Three panes of a parked car in front of the camera, one farther than the others.
    let near = [
        Vec3::new(-0.5, 0.0, -3.0),
        Vec3::new(0.5, 0.0, -4.0),
        Vec3::new(0.0, 0.0, -12.0),
    ];
    let mut s = Scene::new(&near, false);
    s.quality(2, 1);
    run(&mut s, 4);
    assert_eq!(
        ambient_cameras(&mut s).len(),
        2,
        "the budget is the player's mirror count"
    );
    // The two nearest show a picture, the third its own static material.
    let live: Vec<bool> = (0..3)
        .map(|i| material_of(&s, s.mirrors[i]) != s.originals[i])
        .collect();
    assert_eq!(live, [true, true, false]);
    // No player camera is made for them.
    assert!(s.cameras().is_empty());

    // The player's vehicle is left to its own cameras.
    let mut p = Scene::new(&near, true);
    p.quality(2, 1);
    run(&mut p, 4);
    let ambient_on_player = (0..3).any(|i| {
        let pane = p.mirrors[i];
        p.app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(pane)
            .is_some_and(|m| m.0 != p.originals[i])
            && p.app.world().get::<MirrorSurface>(pane).is_none()
    });
    assert!(!ambient_on_player);
}

#[test]
fn panes_out_of_range_behind_the_camera_or_edge_on_get_nothing() {
    let mut s = Scene::new(
        &[
            Vec3::new(0.0, 0.0, -AMBIENT_RANGE_M - 5.0),
            Vec3::new(0.0, 0.0, 3.0),
        ],
        false,
    );
    s.quality(2, 1);
    run(&mut s, 4);
    for i in 0..2 {
        assert_eq!(material_of(&s, s.mirrors[i]), s.originals[i]);
    }
}

#[test]
fn a_pane_that_loses_its_camera_gets_its_static_material_back_and_budget_zero_removes_the_pool() {
    let mut s = Scene::new(&[Vec3::new(0.0, 0.0, -3.0)], false);
    s.quality(1, 1);
    run(&mut s, 4);
    assert_ne!(material_of(&s, s.mirrors[0]), s.originals[0]);
    s.quality(0, 1);
    run(&mut s, 3);
    assert_eq!(material_of(&s, s.mirrors[0]), s.originals[0]);
    assert!(ambient_cameras(&mut s).is_empty());
}

#[test]
fn a_camera_keeps_its_pane_while_it_stays_among_the_nearest() {
    let mut s = Scene::new(
        &[Vec3::new(-0.5, 0.0, -3.0), Vec3::new(0.5, 0.0, -4.0)],
        false,
    );
    s.quality(1, 1);
    run(&mut s, 4);
    let first = material_of(&s, s.mirrors[0]);
    assert_ne!(first, s.originals[0]);
    // A pane that becomes nearer than the served one does not take the camera... until the
    // served pane drops out of the budget.
    let pane = s.mirrors[1];
    s.app
        .world_mut()
        .entity_mut(pane)
        .insert(Transform::from_translation(Vec3::new(0.5, 0.0, -2.0)));
    s.app
        .world_mut()
        .entity_mut(pane)
        .insert(GlobalTransform::from_translation(Vec3::new(0.5, 0.0, -2.0)));
    run(&mut s, 3);
    assert_ne!(material_of(&s, s.mirrors[1]), s.originals[1]);
    assert_eq!(material_of(&s, s.mirrors[0]), s.originals[0]);
}
