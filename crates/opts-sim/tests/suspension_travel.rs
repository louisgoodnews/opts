//! The wheel travel the scripts read as `Axle_Suspension_<n>_<l|r>` (2026-10-26): OMSI's
//! extension of the spring from its unloaded length, negative under load. The air-suspension
//! levelling of the MAN and Valiant brake scripts fills a bellows while the travel is below
//! `bremse_level_setpoint` (-0.105) and drains it while it is above; with the compression fed
//! instead (positive under load) it drained every bellows and the body sank onto its bump stops,
//! leaning to one side.

use opts_sim::vehicle::dynamic_mass_kg;
use opts_sim::VehicleSimulation;

const DT: f32 = 1.0 / 64.0;

fn settled(people: usize) -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(11_000.0, 5.85);
    sim.passenger_mass_kg = dynamic_mass_kg(0.0, people, 75.0);
    sim.inputs.set_parking_brake(true);
    for _ in 0..(4.0 / DT) as usize {
        sim.step(DT);
    }
    sim
}

#[test]
fn a_standing_wheel_reads_its_static_deflection_as_a_negative_travel() {
    let sim = settled(0);
    let body = sim.body.as_ref().expect("body");
    for w in &body.wheels {
        let deflection = w.static_load_n / w.spring_n_m;
        let travel = w.omsi_travel_m();
        assert!(travel < -0.01, "negative under load: {travel}");
        assert!(
            (travel + deflection).abs() < 0.01,
            "{travel} vs {deflection}"
        );
    }
    let mean: f32 = body
        .wheels
        .iter()
        .map(|w| -w.static_load_n / w.spring_n_m)
        .sum::<f32>()
        / body.wheels.len() as f32;
    assert!((body.rest_travel_m() - mean).abs() < 1e-5);
}

#[test]
fn a_loaded_body_reads_lower_and_a_softer_spring_too() {
    let (empty, full) = (settled(0), settled(80));
    let travel = |s: &VehicleSimulation| s.body.as_ref().unwrap().wheels[0].omsi_travel_m();
    assert!(
        travel(&full) < travel(&empty) - 0.005,
        "{} vs {}",
        travel(&full),
        travel(&empty)
    );
    let mut soft = settled(0);
    for k in 0..4 {
        soft.set_spring_factor(k, 0.5);
    }
    for _ in 0..(4.0 / DT) as usize {
        soft.step(DT);
    }
    assert!(travel(&soft) < travel(&empty) - 0.01);
}

/// The levelling loop of the brake scripts, reduced to its valves: the bellows pressure (the
/// spring factor is pressure over 460,000 Pa) rises while the travel is below the set point and
/// falls while it is above. It must settle at the set point, not run to an empty bellows.
#[test]
fn the_scripts_levelling_holds_the_body_at_its_set_point() {
    const SETPOINT: f32 = -0.105;
    const TOLERANCE: f32 = 0.005;
    let mut sim = settled(40);
    let mut pressure = [460_000.0f32; 4];
    for _ in 0..(60.0 / DT) as usize {
        for (k, p) in pressure.iter_mut().enumerate() {
            let travel = sim.body.as_ref().unwrap().wheels[k].omsi_travel_m();
            if travel < SETPOINT - TOLERANCE {
                *p += 60_000.0 * DT;
            } else if travel > SETPOINT + TOLERANCE {
                *p = (*p - 60_000.0 * DT).max(100_000.0);
            }
            sim.set_spring_factor(k, (*p / 460_000.0).max(0.3));
        }
        sim.step(DT);
    }
    let body = sim.body.as_ref().unwrap();
    for (k, w) in body.wheels.iter().enumerate() {
        let travel = w.omsi_travel_m();
        assert!(
            (travel - SETPOINT).abs() < 3.0 * TOLERANCE,
            "wheel {k}: travel {travel:.3}, bellows {:.0} Pa",
            pressure[k]
        );
        assert!(
            pressure[k] > 200_000.0,
            "wheel {k} drained: {:.0} Pa",
            pressure[k]
        );
    }
    assert!(body.roll_rad.abs() < 0.005, "level: roll {}", body.roll_rad);
}
