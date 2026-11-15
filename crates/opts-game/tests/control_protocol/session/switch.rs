//! Switching, despawning and respawning session vehicles.


use super::*;


#[test]
pub(super) fn session_switch_moves_the_driver_to_another_vehicle() {
    let mut app = session_app();
    let a = add_vehicle(&mut app, "Alpha", "Depot", 0.0);
    let b = add_vehicle(&mut app, "Beta", "Stop", 50.0);
    let err = call(&mut app, "session.switch", json!({})).expect_err("needs id or step");
    assert_eq!(err.code, code::INVALID_PARAMS);
    let err = call(&mut app, "session.switch", json!({ "id": 99 })).expect_err("unknown");
    assert_eq!(err.code, code::NOT_FOUND);
    assert_eq!(
        call(&mut app, "session.switch", json!({ "id": a })).expect("a")["active"],
        a
    );
    assert_eq!(
        call(&mut app, "session.switch", json!({ "step": 1 })).expect("next")["active"],
        b
    );
    let mut q = app
        .world_mut()
        .query_filtered::<&SessionVehicle, With<PlayerVehicle>>();
    let active: Vec<u32> = q.iter(app.world()).map(|s| s.0).collect();
    assert_eq!(active, vec![b as u32], "only the new vehicle takes input");
    // The old one is parked.
    let mut q = app
        .world_mut()
        .query_filtered::<(&SessionVehicle, &VehicleController), Without<PlayerVehicle>>();
    let parked = q.iter(app.world()).next().expect("parked");
    assert!(parked.1.inputs().is_parking_brake_engaged());
}

#[test]
pub(super) fn session_despawn_and_respawn_work_on_active_and_inactive_vehicles() {
    let mut app = session_app();
    let a = add_vehicle(&mut app, "Alpha", "Depot", 0.0);
    let b = add_vehicle(&mut app, "Beta", "Stop", 50.0);
    call(&mut app, "session.switch", json!({ "id": b })).expect("b");
    // Respawn of the active vehicle: back at its spawn point.
    {
        let mut q = app
            .world_mut()
            .query_filtered::<&mut VehicleController, With<PlayerVehicle>>();
        q.single_mut(app.world_mut())
            .expect("player")
            .sim
            .chassis
            .position = Vec2::new(400.0, 3.0);
    }
    let r = call(&mut app, "session.respawn", json!({})).expect("respawn active");
    assert_eq!(r["id"], b);
    let mut q = app
        .world_mut()
        .query_filtered::<&VehicleController, With<PlayerVehicle>>();
    assert_eq!(
        q.single(app.world()).expect("p").sim.chassis.position,
        Vec2::new(50.0, 0.0)
    );
    // Despawn the inactive one.
    let out = call(&mut app, "session.despawn", json!({ "id": a })).expect("inactive");
    assert_eq!(out["active"], b);
    assert_eq!(out["removed"], 1);
    let err = call(&mut app, "session.respawn", json!({ "id": a })).expect_err("gone");
    assert_eq!(err.code, code::NOT_READY);
    // Despawn the active one: none is left.
    let out = call(&mut app, "session.despawn", json!({})).expect("active");
    assert_eq!(out["active"], Value::Null);
    let err = call(&mut app, "session.despawn", json!({})).expect_err("nothing to despawn");
    assert_eq!(err.code, code::NOT_READY);
    let list = call(&mut app, "session.vehicles", json!({})).expect("list");
    assert_eq!(list["count"], 0);
    assert_eq!(list["place_label"], Value::Null);
}
