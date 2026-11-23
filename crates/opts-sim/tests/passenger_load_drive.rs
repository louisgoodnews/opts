//! M-0124 criteria 2 and 3: a loaded bus accelerates and brakes by the amounts its mass and the
//! drive model give. Constants are the model's own (cited per line); each expected value is
//! derived in its test.

use opts_sim::vehicle::dynamic_mass_kg;
use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::VehicleSimulation;

const DT: f32 = 1.0 / 64.0;
const G: f32 = 9.81;
const CURB_KG: f32 = 11_000.0;
const PEOPLE: usize = 80; // 80 x 75 kg: 17,000 kg loaded
const M_EMPTY: f32 = CURB_KG;
const M_FULL: f32 = 17_000.0;
const ROAD_MU: f32 = 0.85; // ROAD_FRICTION (vehicle/mod.rs)
const ROLLING: f32 = 0.012; // ROLLING_RESISTANCE_COEFF (drivetrain/model.rs)
const BRAKE_FACTOR: f32 = 0.55; // brakes.rs: an axle brakes at 0.55 x its rated load
const AXLE_RATED_N: f32 = 115_000.0; // axle/config.rs: default max_force_n

fn bus(people: usize) -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(CURB_KG, 5.85);
    sim.passenger_mass_kg = dynamic_mass_kg(0.0, people, 75.0);
    sim
}

/// Drives at full throttle for `secs`, returning (time, speed, distance) after each step.
fn accelerate(sim: &mut VehicleSimulation, secs: f32) -> Vec<(f32, f32, f32)> {
    sim.inputs.set_gear(GearPosition::Drive);
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_throttle(1.0);
    let start = sim.chassis.position;
    let mut trace = Vec::new();
    for i in 1..=(secs / DT) as usize {
        sim.step(DT);
        let d = (sim.chassis.position - start).length();
        trace.push((i as f32 * DT, sim.chassis.forward_speed_m_s(), d));
    }
    trace
}

/// At the moment of the first step the body and its static axle loads exist. Stopping from v0
/// with the service brake at full: each axle gives min(BRAKE_FACTOR x rated, ROAD_MU x load),
/// the brake decel is that sum over the mass, and rolling adds ROLLING x g.
fn stop_decel(sim: &VehicleSimulation) -> f32 {
    let body = sim.body.as_ref().expect("body exists after a step");
    let brake: f32 = (0..sim.axles.len())
        .map(|i| (BRAKE_FACTOR * AXLE_RATED_N).min(ROAD_MU * body.axle_load_n(i)))
        .sum();
    brake / sim.consist_mass_kg() + ROLLING * G
}

/// Stops from `v0` with the service brake at full. Returns (distance, predicted decel).
fn stop_from(people: usize, v0: f32) -> (f32, f32) {
    let mut sim = bus(people);
    sim.inputs.set_gear(GearPosition::Drive);
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_throttle(0.0);
    sim.inputs.set_service_brake(1.0);
    for _ in 0..(2.0 / DT) as usize {
        sim.step(DT); // builds the body and settles its axle loads with the people at rest
    }
    let decel = stop_decel(&sim);
    sim.chassis.local_velocity.y = v0;
    let start = sim.chassis.position;
    for _ in 0..(30.0 / DT) as usize {
        sim.step(DT);
        if sim.chassis.forward_speed_m_s().abs() < 0.01 {
            break;
        }
    }
    ((sim.chassis.position - start).length(), decel)
}

#[test]
fn a_loaded_bus_stops_longer_by_the_ratio_its_brakes_and_grip_give() {
    // Stopping distance = v0^2 / (2 a) for constant decel a. Empty, both axles are grip-limited,
    // so a_e = (mu + c) g = 8.46 m/s^2 whatever the mass; loaded, the rear axle is grip-limited
    // and the front is held to its brake capacity, so a_f = (sum of the two axle forces) / m_f
    // + c g = 6.85 m/s^2. Both decels come from the model's own axle loads (stop_decel), so the
    // expected ratio of distances is a_e / a_f = 1.235. The brake build-up adds a similar lag to
    // both stops, which the 15 % tolerance absorbs (measured 1.240). Mass ratio bounds it: 1.545.
    let v0 = 50.0 / 3.6;
    let (d_empty, a_e) = stop_from(0, v0);
    let (d_full, a_f) = stop_from(PEOPLE, v0);
    let predicted_ratio = a_e / a_f;
    let ratio = d_full / d_empty;
    assert!(
        d_full > d_empty + 1.0,
        "full {d_full:.2} m vs empty {d_empty:.2} m"
    );
    assert!(
        (ratio - predicted_ratio).abs() <= 0.15 * predicted_ratio,
        "ratio {ratio:.3} vs predicted {predicted_ratio:.3} (a_e {a_e:.2}, a_f {a_f:.2})"
    );
    assert!(
        ratio <= M_FULL / M_EMPTY,
        "ratio {ratio:.3} exceeds the mass ratio"
    );
}

#[test]
fn a_loaded_bus_covers_less_ground_in_10_s_on_the_mass_time_scale() {
    // Same engine force at each speed: a = F(v)/m - c g - D/m. Without the rolling term (which
    // scales with the mass too) the loaded bus at time T has the empty bus's speed at T m_e/m_f,
    // so its distance is (m_f/m_e) times the empty bus's distance at T m_e/m_f (43.5 m x 1.545 =
    // 67.2 m; measured 65.6 m). The rolling term is the tolerance.
    let (mut empty, mut full) = (bus(0), bus(PEOPLE));
    let e = accelerate(&mut empty, 10.0);
    let f = accelerate(&mut full, 10.0);
    // Distance is the integral of speed, so the stretch in time adds the factor M_FULL / M_EMPTY.
    let scaled = e[(10.0 * M_EMPTY / M_FULL / DT) as usize - 1].2 * M_FULL / M_EMPTY;
    let (_, _, d_full) = f[f.len() - 1];
    let (_, _, d_empty) = e[e.len() - 1];
    assert!(
        d_full < d_empty,
        "full {d_full:.1} m vs empty {d_empty:.1} m in 10 s"
    );
    assert!(
        (d_full - scaled).abs() <= 0.15 * scaled,
        "full {d_full:.1} m vs empty-at-6.47 s {scaled:.1} m"
    );
}
