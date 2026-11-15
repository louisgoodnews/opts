//! Service and parking brake: stopping distances, the pneumatic build-up and the grade hold.

use super::support::*;
use opts_sim::vehicle::ground::UnitBody;
use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::VehicleSimulation;

#[test]
fn the_brakes_are_sized_for_the_rated_load_so_a_loaded_bus_stops_longer_than_a_car() {
    let car = stopping_distance(
        at_speed(
            VehicleSimulation::from_manifest(&file(1.2, 0.55, 1.3, 8.0)),
            KMH_50,
        ),
        0.01,
    );
    // The bus carries 28 t on axles rated for 320 kN: 0.55 g at the rated load, so about 6.3 m/s²
    // at this one, less than the road allows the car.
    let bus = stopping_distance(at_speed(articulated(0.0), KMH_50), 0.01);
    eprintln!("stopping from 50 km/h: car {car:.1} m, loaded articulated bus {bus:.1} m");
    // From 50 km/h: a car on dry asphalt about 12 to 15 m with the brakes' build-up, a loaded
    // articulated bus about 17 to 23 m.
    assert!((10.0..16.0).contains(&car), "car {car:.1} m");
    assert!((15.0..25.0).contains(&bus), "bus {bus:.1} m");
    assert!(bus > car * 1.15, "bus {bus:.1} m, car {car:.1} m");
    // More load on the same brakes: longer still.
    let heavier = stopping_distance(at_speed(articulated(3_000.0), KMH_50), 0.01);
    assert!(heavier > bus, "{heavier:.1} m against {bus:.1} m");
}

#[test]
fn the_pneumatic_build_up_costs_distance() {
    let sim = || {
        at_speed(
            VehicleSimulation::from_manifest(&file(10.0, 1.0, 3.0, 100.0)),
            KMH_50,
        )
    };
    let lagged = stopping_distance(sim(), 0.005);
    // The same brake with a pressure that is there at once.
    let mut instant = sim();
    instant.brakes.pressure = 1.0;
    let start = instant.chassis.position.y;
    instant.inputs.set_service_brake(1.0);
    for _ in 0..4000 {
        instant.step(0.005);
        if instant.chassis.forward_speed_m_s() == 0.0 {
            break;
        }
    }
    let direct = instant.chassis.position.y - start;
    // Roughly the speed times the time constant of the build-up (0.3 s) is lost.
    assert!(
        lagged - direct > 2.0,
        "lagged {lagged:.1} m, direct {direct:.1} m"
    );
}

#[test]
fn the_stopping_distance_does_not_depend_on_the_step_size() {
    let sim = || {
        at_speed(
            VehicleSimulation::from_manifest(&file(10.0, 1.0, 3.0, 100.0)),
            KMH_50,
        )
    };
    let (fine, coarse) = (
        stopping_distance(sim(), 1.0 / 200.0),
        stopping_distance(sim(), 1.0 / 50.0),
    );
    assert!(
        (fine - coarse).abs() <= 0.04 * fine,
        "{fine:.2} m against {coarse:.2} m"
    );
}

#[test]
fn the_parking_brake_holds_on_a_ten_percent_grade_and_gives_way_on_a_steep_one() {
    let hold_on = |slope: f32, parking: bool| {
        let mut sim = VehicleSimulation::from_manifest(&file(10.0, 1.0, 3.0, 100.0));
        sim.inputs.set_parking_brake(parking);
        sim.inputs.set_gear(GearPosition::Neutral);
        sim.head_body = Some(UnitBody {
            elevation: 0.0,
            pitch: slope.atan(),
            roll: 0.0,
        });
        for _ in 0..1000 {
            sim.step(0.01);
        }
        sim.chassis.forward_speed_m_s()
    };
    assert_eq!(hold_on(0.10, true), 0.0, "held on 10 %");
    assert_eq!(hold_on(0.30, true), 0.0, "held on 30 %");
    assert!(
        hold_on(0.70, true) < -1.0,
        "rolls backwards on 70 %: the spring brake holds 0.5 g"
    );
    // Without the brake the same 10 % slope sets it rolling (gravity 0.1 g against 0.012 g).
    assert!(hold_on(0.10, false) < -3.0, "no brake, no hold");
}
