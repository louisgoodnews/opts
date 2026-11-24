//! Braking margin of the dynamic mass (M-0124 criterion 3): the steady brake deceleration per
//! axle and the 50 km/h stop, with the expected values from the brake parameters
//! (`BRAKE_FACTOR`, the rated force of each axle, `ROAD_FRICTION`) and the mass.
use opts_sim::vehicle::brakes::BRAKE_FACTOR;
use opts_sim::vehicle::dynamic_mass_kg;
use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::VehicleSimulation;

const DT: f32 = 1.0 / 64.0;
const CURB_KG: f32 = 11_000.0;
/// Average passenger mass (kg): the setting's default.
const AVERAGE_KG: f32 = 75.0;
/// People in the loaded bus (the rated capacity is not in the files, see the card's audit).
const LOADED_PEOPLE: usize = 80;
const G: f32 = 9.81;
/// M-0124 criterion 3 (owner decision 2026-11-24): the loaded stop from 50 km/h is at least this
/// much longer than the empty stop.
const MIN_LONGER_STOP_FRACTION: f32 = 0.20;
/// Recorded baseline (owner decision 2026-11-24): the empty bus's stop from 50 km/h, measured on
/// 2026-11-24 with the brake parameters of `BRAKE_FACTOR`, the axles' rated force and
/// `ROAD_FRICTION` (no passengers, the setting off). The empty stop must stay within 1 % of it.
const EMPTY_STOP_50_KMH_BASELINE_M: f32 = 13.7742;
/// Intended behaviour (owner decision 2026-11-24): at 100 km/h the loaded bus brakes about 19 %
/// softer than the empty bus, because its rear axle is held at the tyre limit, not the brake cap.
const LOADED_SOFTER_FRACTION: f32 = 0.19;

fn bus(people: usize) -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(CURB_KG, 5.85);
    sim.passenger_mass_kg = dynamic_mass_kg(0.0, people, AVERAGE_KG);
    sim
}

// Mass the bus must move, from the parameters only (not from the sim).

fn expected_mass(people: usize) -> f32 {
    dynamic_mass_kg(CURB_KG, people, AVERAGE_KG)
}

fn brake_from(sim: &mut VehicleSimulation, kmh: f32) {
    sim.inputs.set_gear(GearPosition::Drive);
    sim.inputs.set_parking_brake(false);
    sim.chassis.local_velocity.y = kmh / 3.6;
    sim.inputs.set_throttle(0.0);
    sim.inputs.set_service_brake(1.0);
}

fn decel_from(sim: &mut VehicleSimulation, kmh: f32) -> (f32, Vec<f32>, f32, f32) {
    brake_from(sim, kmh);
    let mut samples = Vec::new();
    for step in 0..(2.4 / DT) as usize {
        sim.step(DT);
        let t = (step + 1) as f32 * DT;
        if t >= 2.0 {
            let body = sim.body.as_ref().unwrap();
            let loads: Vec<f32> = (0..sim.axles.len()).map(|i| body.axle_load_n(i)).collect();
            samples.push((t, sim.chassis.forward_speed_m_s(), loads));
        }
    }
    let (t0, v0, l0) = samples[0].clone();
    let (t1, v1, l1) = samples.last().unwrap().clone();
    let loads = l0.iter().zip(&l1).map(|(a, b)| 0.5 * (a + b)).collect();
    (t1 - t0, loads, v0, v1)
}

#[test]
fn the_steady_brake_decel_is_the_capped_or_tyre_limited_force_over_the_expected_mass() {
    // After the pressure has built up (2 s at 1 - e^(-2/0.30) = 0.9989), per axle:
    // F_i = min(BRAKE_FACTOR * rated_i, mu * load_i), a = (sum F_i + c_r * m * g + drag) / m.
    for people in [0, LOADED_PEOPLE] {
        let mut sim = bus(people);
        let m = expected_mass(people);
        let (dt_win, loads, v0, v1) = decel_from(&mut sim, 100.0);
        let caps: Vec<f32> = sim
            .axles
            .iter()
            .map(|a| BRAKE_FACTOR * a.config.max_force_n)
            .collect();
        let forces: Vec<f32> = caps
            .iter()
            .zip(&loads)
            .map(|(c, l)| c.min(sim.road_friction * l))
            .collect();
        let v_mid = 0.5 * (v0 + v1);
        let drag = sim.drivetrain.aero.compute_drag_force(v_mid);
        let rolling = sim.drivetrain.rolling_coefficient * m * G;
        let expected = (forces.iter().sum::<f32>() + rolling + drag) / m;
        let measured = (v0 - v1) / dt_win;
        eprintln!(
            "people {people}: m {m:.0} kg, axle loads {loads:.0?}, caps {caps:.0?}, forces {forces:.0?}, rolling {rolling:.0} N, drag {drag:.0} N, decel expected {expected:.4} m/s2, measured {measured:.4} m/s2"
        );
        assert!(
            (measured - expected).abs() <= 0.02 * expected,
            "people {people}: decel {measured:.4} vs {expected:.4} m/s2"
        );
    }
}

#[test]
fn a_loaded_bus_stops_longer_from_fifty_but_by_less_than_its_mass_ratio() {
    let mut empty = bus(0);
    let mut loaded = bus(LOADED_PEOPLE);
    let stop = |sim: &mut VehicleSimulation| {
        brake_from(sim, 50.0);
        let start = sim.chassis.position;
        let mut t = 0.0;
        while sim.chassis.forward_speed_m_s() > 0.0 && t < 60.0 {
            sim.step(DT);
            t += DT;
        }
        ((sim.chassis.position - start).length(), t)
    };
    let (d_e, t_e) = stop(&mut empty);
    let (d_l, t_l) = stop(&mut loaded);
    let mass_ratio = expected_mass(LOADED_PEOPLE) / expected_mass(0);
    eprintln!(
        "50 km/h stop: empty {d_e:.4} m in {t_e:.2} s; loaded {d_l:.4} m in {t_l:.2} s; ratio {:.4}; mass ratio {mass_ratio:.4}",
        d_l / d_e
    );
    assert!(d_l > d_e, "loaded {d_l:.2} m, empty {d_e:.2} m");
    assert!(
        d_l <= mass_ratio * d_e,
        "ratio {:.4} above the mass ratio {mass_ratio:.4}",
        d_l / d_e
    );
    // Criterion 3 thresholds (owner decision 2026-11-24).
    let longer = d_l / d_e - 1.0;
    assert!(
        longer >= MIN_LONGER_STOP_FRACTION,
        "loaded stop {:.1} % longer, threshold {:.0} %",
        100.0 * longer,
        100.0 * MIN_LONGER_STOP_FRACTION
    );
    // Recorded empty baseline (within 1 %).
    assert!(
        (d_e - EMPTY_STOP_50_KMH_BASELINE_M).abs() <= 0.01 * EMPTY_STOP_50_KMH_BASELINE_M,
        "empty stop {d_e:.4} m vs recorded baseline {EMPTY_STOP_50_KMH_BASELINE_M} m"
    );
}

#[test]
fn the_loaded_bus_brakes_softer_at_100_kmh_because_its_rear_axle_is_tyre_limited() {
    // Intended (owner decision 2026-11-24, card note): the loaded rear axle is held at its tyre
    // limit mu * load_rear, below its brake cap, so the loaded bus decelerates about 19 % less
    // than the empty one. Pinned so that a change to this behaviour is a visible decision.
    let mut empty = bus(0);
    let mut loaded = bus(LOADED_PEOPLE);
    let (dt_e, loads_e, v0_e, v1_e) = decel_from(&mut empty, 100.0);
    let (dt_l, loads_l, v0_l, v1_l) = decel_from(&mut loaded, 100.0);
    let decel_e = (v0_e - v1_e) / dt_e;
    let decel_l = (v0_l - v1_l) / dt_l;
    let cap_rear = BRAKE_FACTOR * empty.axles[1].config.max_force_n;
    let rear_force_l = loaded.road_friction * loads_l[1];
    eprintln!(
        "100 km/h: empty decel {decel_e:.4} m/s2 (rear load {:.0} N), loaded decel {decel_l:.4} m/s2 (rear load {:.0} N, rear tyre limit {rear_force_l:.0} N, rear cap {cap_rear:.0} N)",
        loads_e[1], loads_l[1]
    );
    // The loaded rear axle is tyre-limited: below its cap, and at mu times its load.
    assert!(rear_force_l < cap_rear, "loaded rear axle reaches its cap");
    let softer = 1.0 - decel_l / decel_e;
    assert!(
        (softer - LOADED_SOFTER_FRACTION).abs() <= 0.01,
        "loaded decel {softer:.4} softer than empty, pinned {LOADED_SOFTER_FRACTION}"
    );
}

#[test]
fn with_no_people_the_stop_is_the_curb_mass_stop() {
    let mut a = VehicleSimulation::new_standard_bus(CURB_KG, 5.85);
    let mut b = bus(0);
    brake_from(&mut a, 50.0);
    brake_from(&mut b, 50.0);
    for _ in 0..(10.0 / DT) as usize {
        a.step(DT);
        b.step(DT);
    }
    assert_eq!(a.chassis.position, b.chassis.position);
    assert_eq!(a.consist_mass_kg(), CURB_KG);
}
