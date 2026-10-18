//! M-0073: a vehicle behaves as its own file says. Two vehicles that differ only in a stated
//! value (rolling resistance, centre of gravity height, mass) differ measurably in the same
//! scenario, and a vehicle that states nothing keeps the defaults.

use glam::Vec2;
use opts_core::formats::vehicles::bus::BusManifest;
use opts_sim::vehicle::spec::{Source, DEFAULT_CG_HEIGHT_M};
use opts_sim::VehicleSimulation;

/// A two-axle road vehicle file with the given stated physics.
fn vehicle(mass_t: f32, cg: f32, roll_n: f32) -> BusManifest {
    BusManifest::parse(&format!(
        "[mass]\n{mass_t}\n0\n0\n0\n\n[schwerpunkt]\n{cg}\n\n[rollwiderstand]\n{roll_n}\n\n\
         [newachse]\nachse_long\n3.0\nachse_raddurchmesser\n0.9\nachse_feder\n250\n\
         achse_maxforce\n100\nachse_daempfer\n20\n\n\
         [newachse]\nachse_long\n-3.0\nachse_raddurchmesser\n0.9\nachse_feder\n250\n\
         achse_maxforce\n100\nachse_daempfer\n20\nachse_antrieb\n1\n"
    ))
    .expect("parses")
}

/// Distance (m) coasted in 20 s from 15 m/s with no driver input.
fn coast_distance(manifest: &BusManifest) -> f32 {
    let mut sim = VehicleSimulation::from_manifest(manifest);
    sim.inputs.set_parking_brake(false);
    sim.chassis.local_velocity = Vec2::new(0.0, 15.0);
    let start = sim.chassis.position;
    for _ in 0..(20 * 100) {
        sim.step(0.01);
    }
    (sim.chassis.position - start).length()
}

#[test]
fn the_stated_rolling_resistance_decides_how_far_a_vehicle_coasts() {
    // Same mass, same everything, 3 times the rolling resistance.
    let free = coast_distance(&vehicle(10.0, 1.0, 300.0));
    let sticky = coast_distance(&vehicle(10.0, 1.0, 900.0));
    assert!(
        free > sticky * 1.02,
        "300 N coasts {free:.1} m, 900 N coasts {sticky:.1} m"
    );
}

#[test]
fn the_stated_centre_of_gravity_decides_the_load_transfer_in_braking() {
    let braking = |cg: f32| {
        let sim = VehicleSimulation::from_manifest(&vehicle(10.0, cg, 800.0));
        let positions: Vec<f32> = sim.axles.iter().map(|a| a.config.longitudinal_m).collect();
        let stat = sim
            .suspension
            .compute_static_loads(&positions, sim.chassis.total_mass_kg);
        let dynamic = sim.suspension.compute_dynamic_loads(
            &stat,
            &positions,
            sim.chassis.total_mass_kg,
            -5.0,
        );
        dynamic[0] - stat[0]
    };
    let (low, high) = (braking(0.55), braking(1.4));
    assert!(
        high > low * 2.0,
        "extra front load {low:.0} N at 0.55 m, {high:.0} N at 1.4 m"
    );
}

#[test]
fn a_vehicle_that_states_nothing_keeps_the_generic_bus() {
    let m = BusManifest::parse("[friendlyname]\nA\nB\nC\n").expect("parses");
    let sim = VehicleSimulation::from_manifest(&m);
    assert_eq!(sim.spec.cg_height_m.source, Source::Default);
    assert_eq!(sim.suspension.cg_height_m, DEFAULT_CG_HEIGHT_M);
    assert_eq!(sim.chassis.total_mass_kg, 11_500.0);
}

#[test]
fn the_spec_reaches_the_simulation_and_names_its_sources() {
    let sim = VehicleSimulation::from_manifest(&vehicle(1.2, 0.55, 100.0));
    assert_eq!(sim.chassis.total_mass_kg, 1_200.0);
    assert_eq!(sim.suspension.cg_height_m, 0.55);
    assert_eq!(sim.spec.mass_kg.source, Source::File);
    assert_eq!(sim.spec.cg_height_m.source, Source::File);
    assert_eq!(sim.spec.rolling_coefficient.source, Source::File);
    assert!((sim.drivetrain.rolling_coefficient - 100.0 / (1_200.0 * 9.81)).abs() < 1e-6);
}
