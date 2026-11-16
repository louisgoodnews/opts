//! The place label follows the spawn place of the active vehicle.

use super::*;

#[test]
fn the_place_label_follows_the_spawn_place() {
    let mut app = app();
    assert_eq!(
        app.world().resource::<PlaceLabel>().0,
        None,
        "no vehicle, no label"
    );
    let (a, ea) = add(&mut app, "A", "Rathaus", Vec2::new(10.0, 10.0));
    ops::switch_to(app.world_mut(), a).expect("a");
    run(&mut app, 2);
    assert_eq!(
        app.world().resource::<PlaceLabel>().0.as_deref(),
        Some("Rathaus")
    );
    // A few metres: still there.
    app.world_mut()
        .get_mut::<VehicleController>(ea)
        .expect("c")
        .sim
        .chassis
        .position = Vec2::new(20.0, 10.0);
    run(&mut app, 2);
    assert_eq!(
        app.world().resource::<PlaceLabel>().0.as_deref(),
        Some("Rathaus")
    );
    // Further than the threshold: gone, and it stays gone when the vehicle comes back.
    app.world_mut()
        .get_mut::<VehicleController>(ea)
        .expect("c")
        .sim
        .chassis
        .position = Vec2::new(10.0 + PLACE_LABEL_HIDE_DISTANCE_M + 5.0, 10.0);
    run(&mut app, 2);
    assert_eq!(app.world().resource::<PlaceLabel>().0, None);
    app.world_mut()
        .get_mut::<VehicleController>(ea)
        .expect("c")
        .sim
        .chassis
        .position = Vec2::new(10.0, 10.0);
    run(&mut app, 2);
    assert_eq!(
        app.world().resource::<PlaceLabel>().0,
        None,
        "moved away once: hidden"
    );
    // A respawn shows it again.
    ops::respawn(app.world_mut(), a).expect("respawn");
    run(&mut app, 2);
    assert_eq!(
        app.world().resource::<PlaceLabel>().0.as_deref(),
        Some("Rathaus")
    );
    // A new vehicle spawned at another place shows its own name once active; despawn hides it.
    let (b, _) = add(&mut app, "B", "Bahnhof", Vec2::new(500.0, 0.0));
    ops::switch_to(app.world_mut(), b).expect("b");
    run(&mut app, 2);
    assert_eq!(
        app.world().resource::<PlaceLabel>().0.as_deref(),
        Some("Bahnhof")
    );
    ops::despawn(app.world_mut(), b, DespawnReason::User).expect("despawn b");
    ops::despawn(app.world_mut(), a, DespawnReason::User).expect("despawn a");
    run(&mut app, 2);
    assert_eq!(app.world().resource::<PlaceLabel>().0, None);
}
