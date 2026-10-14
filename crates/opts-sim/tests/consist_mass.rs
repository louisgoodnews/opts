//! A consist carries the mass of all its units: it accelerates like a solo vehicle of the same
//! total mass, not like its head alone, and traction counts the mass on driven axles.

use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::{UnitLoad, VehicleSimulation};

const DT: f32 = 1.0 / 64.0;

fn solo(mass_kg: f32) -> VehicleSimulation {
    VehicleSimulation::new_standard_bus(mass_kg, 5.85)
}

fn consist(head_kg: f32, trailer_kg: f32, trailer_driven_axles: usize) -> VehicleSimulation {
    let mut sim = solo(head_kg);
    sim.set_trailers(vec![UnitLoad {
        mass_kg: trailer_kg,
        axles: 2,
        driven_axles: trailer_driven_axles,
    }]);
    sim
}

fn full_throttle(sim: &mut VehicleSimulation, seconds: f32) -> f32 {
    sim.inputs.set_gear(GearPosition::Drive);
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_throttle(1.0);
    for _ in 0..(seconds / DT) as usize {
        sim.step(DT);
    }
    sim.chassis.forward_speed_m_s()
}

/// Matches the NG272 manifests the census measured (head 8.55 t, trailer 7.4 t: ratio 0.87).
#[test]
fn a_consist_accelerates_like_a_solo_vehicle_of_the_same_total_mass() {
    let (head, trailer) = (8_550.0, 7_400.0);
    let mut together = consist(head, trailer, 0);
    let mut equal = solo(head + trailer);
    let mut head_only = solo(head);
    assert_eq!(together.consist_mass_kg(), head + trailer);
    let (v_together, v_equal, v_head) = (
        full_throttle(&mut together, 6.0),
        full_throttle(&mut equal, 6.0),
        full_throttle(&mut head_only, 6.0),
    );
    assert!(
        (v_together - v_equal).abs() <= 0.02 * v_equal,
        "consist {v_together:.2} m/s vs solo of equal mass {v_equal:.2} m/s"
    );
    assert!(
        v_head > v_together * 1.2,
        "the head alone ({v_head:.2} m/s) must be clearly faster than the consist ({v_together:.2} m/s)"
    );
}

#[test]
fn a_consist_stops_like_a_solo_vehicle_of_the_same_total_mass() {
    let stop = |mut sim: VehicleSimulation| {
        full_throttle(&mut sim, 5.0);
        let start = sim.chassis.position.y;
        sim.inputs.set_throttle(0.0);
        sim.inputs.set_service_brake(1.0);
        for _ in 0..(10.0 / DT) as usize {
            sim.step(DT);
        }
        assert_eq!(sim.chassis.forward_speed_m_s(), 0.0, "must come to rest");
        sim.chassis.position.y - start
    };
    let (a, b) = (stop(consist(8_550.0, 7_400.0, 0)), stop(solo(15_950.0)));
    assert!((a - b).abs() <= 0.02 * b, "{a:.2} m vs {b:.2} m");
}

#[test]
fn traction_counts_the_mass_on_driven_axles() {
    let undriven = consist(8_550.0, 7_400.0, 0);
    let driven = consist(8_550.0, 7_400.0, 2);
    let head_share = undriven.driven_mass_kg();
    assert!(head_share < undriven.consist_mass_kg(), "{head_share}");
    assert!(
        (driven.driven_mass_kg() - head_share - 7_400.0).abs() < 1.0,
        "a fully driven trailer adds its whole mass"
    );
    // A solo vehicle is unchanged: no following unit.
    assert_eq!(solo(11_500.0).consist_mass_kg(), 11_500.0);
    assert_eq!(solo(11_500.0).head_mass_kg(), 11_500.0);
}

#[test]
fn the_head_keeps_its_own_axle_loads_and_setting_units_again_replaces_them() {
    let mut sim = consist(8_550.0, 7_400.0, 0);
    assert_eq!(sim.head_mass_kg(), 8_550.0);
    sim.set_trailers(Vec::new());
    assert_eq!(sim.consist_mass_kg(), 8_550.0);
    sim.set_trailers(vec![
        UnitLoad {
            mass_kg: 5_000.0,
            axles: 1,
            driven_axles: 0,
        },
        UnitLoad {
            mass_kg: 5_000.0,
            axles: 1,
            driven_axles: 0,
        },
    ]);
    assert_eq!(sim.consist_mass_kg(), 18_550.0);
}
