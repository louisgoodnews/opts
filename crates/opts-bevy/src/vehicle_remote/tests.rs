//! Tests of the remote controls: held values, one-shot requests and releasing.

use super::*;


#[test]
fn held_values_override_and_expire_in_simulation_time() {
    let mut remote = RemoteControls {
        throttle: Some(Hold {
            value: 0.6,
            remaining: Some(0.25),
        }),
        steer: Some(Hold {
            value: -1.0,
            remaining: None,
        }),
        ..Default::default()
    };
    let mut inputs = VehicleInputs::new();
    remote.apply(&mut inputs, 0.1);
    assert!((inputs.throttle() - 0.6).abs() < 1e-6);
    assert!((inputs.steering() + 1.0).abs() < 1e-6);
    remote.apply(&mut inputs, 0.1);
    assert!(remote.throttle.is_some());
    remote.apply(&mut inputs, 0.1);
    assert!(
        remote.throttle.is_none(),
        "0.3 s have run out the 0.25 s hold"
    );
    assert!(remote.steer.is_some(), "an open-ended hold stays");
}

#[test]
fn gear_and_parking_brake_are_one_shot() {
    let mut remote = RemoteControls {
        gear: Some(GearPosition::Drive),
        parking: Some(ParkingRequest::Set(false)),
        ..Default::default()
    };
    let mut inputs = VehicleInputs::new();
    assert!(inputs.is_parking_brake_engaged());
    remote.apply(&mut inputs, 0.016);
    assert_eq!(inputs.gear(), GearPosition::Drive);
    assert!(!inputs.is_parking_brake_engaged());
    assert!(!remote.is_active());
    remote.parking = Some(ParkingRequest::Toggle);
    remote.apply(&mut inputs, 0.016);
    assert!(inputs.is_parking_brake_engaged());
}

#[test]
fn release_all_clears_everything() {
    let mut remote = RemoteControls {
        brake: Some(Hold {
            value: 1.0,
            remaining: None,
        }),
        gear: Some(GearPosition::Reverse),
        ..Default::default()
    };
    assert!(remote.is_active());
    remote.release_all();
    assert!(!remote.is_active());
}
