//! Switching the active vehicle: the player marker moves and the previous vehicle is parked.

use super::*;

#[test]
fn switching_moves_the_player_marker_and_parks_the_previous_vehicle() {
    let mut app = app();
    let (a, ea) = add(&mut app, "A", "Depot", Vec2::ZERO);
    let (b, eb) = add(&mut app, "B", "Stop", Vec2::new(100.0, 0.0));
    ops::switch_to(app.world_mut(), a).expect("switch to a");
    assert_eq!(players(&mut app), vec![ea]);
    app.world_mut()
        .get_mut::<VehicleController>(ea)
        .expect("controller")
        .inputs_mut()
        .set_throttle(0.8);
    ops::switch_to(app.world_mut(), b).expect("switch to b");
    assert_eq!(players(&mut app), vec![eb], "exactly one active vehicle");
    let old = app.world().get::<VehicleController>(ea).expect("a");
    assert!(
        old.inputs().is_parking_brake_engaged(),
        "the previous vehicle is parked"
    );
    assert_eq!(old.inputs().throttle(), 0.0);
    assert_eq!(
        app.world().resource::<SessionVehicles>().active_id(),
        Some(b)
    );
    assert_eq!(app.world().resource::<WorldStats>().vehicle_name, "B");
    // The same vehicle again changes nothing; a gone one is refused.
    ops::switch_to(app.world_mut(), b).expect("no-op");
    assert!(matches!(
        ops::switch_to(app.world_mut(), 99),
        Err(OpError::NotFound(_))
    ));
}

#[test]
fn quick_switch_cycles_through_the_living_vehicles() {
    let mut app = app();
    let (a, _) = add(&mut app, "A", "x", Vec2::ZERO);
    let (b, _) = add(&mut app, "B", "x", Vec2::ZERO);
    let (c, _) = add(&mut app, "C", "x", Vec2::ZERO);
    ops::switch_to(app.world_mut(), a).expect("a");
    assert_eq!(ops::switch_step(app.world_mut(), 1), Ok(b));
    assert_eq!(ops::switch_step(app.world_mut(), 1), Ok(c));
    assert_eq!(ops::switch_step(app.world_mut(), 1), Ok(a));
    assert_eq!(ops::switch_step(app.world_mut(), -1), Ok(c));
    // The key does it: F8 and Shift+F8.
    key(&mut app, QUICK_SWITCH_KEY, bevy::input::keyboard::Key::F8);
    assert_eq!(
        app.world().resource::<SessionVehicles>().active_id(),
        Some(a)
    );
    key(
        &mut app,
        KeyCode::ShiftLeft,
        bevy::input::keyboard::Key::Shift,
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    key(&mut app, QUICK_SWITCH_KEY, bevy::input::keyboard::Key::F8);
    assert_eq!(
        app.world().resource::<SessionVehicles>().active_id(),
        Some(c)
    );
}
