//! Acceleration margin of the dynamic mass (M-0124 criterion 2): from rest at full throttle the
//! expected values come from the mass, the drivetrain (`DrivetrainModel::step`) and the rolling
//! resistance; see `passenger_mass_braking.rs` for the brake side.
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

fn bus(people: usize) -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(CURB_KG, 5.85);
    sim.passenger_mass_kg = dynamic_mass_kg(0.0, people, AVERAGE_KG);
    sim
}

// Mass the bus must move, from the parameters only (not from the sim).

fn expected_mass(people: usize) -> f32 {
    dynamic_mass_kg(CURB_KG, people, AVERAGE_KG)
}

fn drive_full_throttle(sim: &mut VehicleSimulation) {
    sim.inputs.set_gear(GearPosition::Drive);
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_throttle(1.0);
}

fn wheel_radius(sim: &VehicleSimulation) -> f32 {
    sim.axles[0].config.wheel_diameter_m * 0.5
}

// Reference acceleration from rest at full throttle: net force `F_t - drag - rolling` on the
// expected mass, semi-implicit Euler, like `ChassisState::integrate_with_mass`.
// Returns (speed m/s, distance m) after `seconds`.

fn reference_full_throttle(people: usize, seconds: f32) -> (f32, f32) {
    let fresh = bus(people);
    let m = expected_mass(people);
    let mut model = fresh.drivetrain.clone();
    let mut inputs = fresh.inputs;
    inputs.set_gear(GearPosition::Drive);
    inputs.set_parking_brake(false);
    inputs.set_throttle(1.0);
    let (mut v, mut x) = (0.0_f32, 0.0_f32);
    for _ in 0..(seconds / DT) as usize {
        let f = model.step(&inputs, v, wheel_radius(&fresh), m);
        let net = f.tractive_force_n - f.drag_force_n - f.rolling_resistance_n;
        v += net / m * DT;
        x += v * DT;
    }
    (v, x)
}

fn full_throttle(sim: &mut VehicleSimulation, seconds: f32) -> (f32, f32) {
    drive_full_throttle(sim);
    let start = sim.chassis.position;
    for _ in 0..(seconds / DT) as usize {
        sim.step(DT);
    }
    (
        sim.chassis.forward_speed_m_s(),
        (sim.chassis.position - start).length(),
    )
}

#[test]
fn the_first_step_from_rest_accelerates_by_the_force_over_the_expected_mass() {
    // a0 = (F_t - c_r * m * g) / m, with F_t from the drivetrain at standstill (mass-free).
    for people in [0, LOADED_PEOPLE] {
        let mut sim = bus(people);
        let m = expected_mass(people);
        let mut probe = sim.drivetrain.clone();
        let mut inputs = sim.inputs;
        inputs.set_gear(GearPosition::Drive);
        inputs.set_throttle(1.0);
        inputs.set_parking_brake(false);
        let f_t = probe
            .step(&inputs, 0.0, wheel_radius(&sim), m)
            .tractive_force_n;
        let c_r = sim.drivetrain.rolling_coefficient;
        let expected = (f_t - c_r * m * G) / m;

        drive_full_throttle(&mut sim);
        sim.step(DT);
        let measured = sim.chassis.forward_speed_m_s() / DT;
        eprintln!(
            "people {people}: m {m:.0} kg, F_t {f_t:.0} N, a0 expected {expected:.4} m/s2, measured {measured:.4} m/s2"
        );
        assert!(
            (measured - expected).abs() <= 0.01 * expected.abs(),
            "people {people}: a0 {measured:.4} vs {expected:.4} m/s2"
        );
    }
}

#[test]
fn a_loaded_bus_covers_less_ground_in_ten_seconds_as_the_mass_model_gives() {
    let mut empty = bus(0);
    let mut loaded = bus(LOADED_PEOPLE);
    let (v_e, d_e) = full_throttle(&mut empty, 10.0);
    let (v_l, d_l) = full_throttle(&mut loaded, 10.0);
    let (rv_e, rd_e) = reference_full_throttle(0, 10.0);
    let (rv_l, rd_l) = reference_full_throttle(LOADED_PEOPLE, 10.0);
    eprintln!(
        "10 s full throttle: empty {d_e:.2} m ({v_e:.3} m/s) ref {rd_e:.2} m; loaded {d_l:.2} m ({v_l:.3} m/s) ref {rd_l:.2} m; difference {:.2} m",
        d_e - d_l
    );
    assert!(
        (d_e - rd_e).abs() <= 0.01 * rd_e,
        "empty {d_e:.2} vs ref {rd_e:.2}"
    );
    assert!(
        (d_l - rd_l).abs() <= 0.01 * rd_l,
        "loaded {d_l:.2} vs ref {rd_l:.2}"
    );
    assert!((v_e - rv_e).abs() <= 0.01 * rv_e && (v_l - rv_l).abs() <= 0.01 * rv_l);
    assert!(d_l < d_e, "loaded {d_l:.2} m, empty {d_e:.2} m");
}
