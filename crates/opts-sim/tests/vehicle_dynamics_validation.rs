//! M-0073 validation: braking, riding and holding of vehicles that state different masses, axle
//! layouts and ratings, against plausible orders of magnitude (documented in
//! `docs/src/simulation/vehicle-dynamics.md`).

use glam::Vec2;
use opts_core::formats::vehicles::bus::BusManifest;
use opts_sim::vehicle::ground::UnitBody;
use opts_sim::vehicle::inputs::GearPosition;
use opts_sim::{UnitLoad, VehicleSimulation};

const KMH_50: f32 = 50.0 / 3.6;

/// A two-axle vehicle file: mass in t, centre of gravity, wheelbase 2 x `half`, axles rated for
/// `rated_kn` each.
fn file(mass_t: f32, cg: f32, half: f32, rated_kn: f32) -> BusManifest {
    BusManifest::parse(&format!(
        "[mass]\n{mass_t}\n0\n0\n0\n\n[schwerpunkt]\n{cg}\n\n\
         [newachse]\nachse_long\n{half}\nachse_raddurchmesser\n0.7\nachse_feder\n150\n\
         achse_maxforce\n{rated_kn}\nachse_daempfer\n12\n\n\
         [newachse]\nachse_long\n-{half}\nachse_raddurchmesser\n0.7\nachse_feder\n150\n\
         achse_maxforce\n{rated_kn}\nachse_daempfer\n12\nachse_antrieb\n1\n"
    ))
    .expect("parses")
}

fn at_speed(mut sim: VehicleSimulation, v: f32) -> VehicleSimulation {
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_gear(GearPosition::Neutral);
    sim.chassis.local_velocity = Vec2::new(0.0, v);
    sim
}

/// Distance (m) to a standstill from `v` with the service brake fully applied, stepping `dt`.
fn stopping_distance(mut sim: VehicleSimulation, dt: f32) -> f32 {
    let start = sim.chassis.position.y;
    sim.inputs.set_service_brake(1.0);
    for _ in 0..(20.0 / dt) as usize {
        sim.step(dt);
        if sim.chassis.forward_speed_m_s() == 0.0 {
            break;
        }
    }
    assert_eq!(sim.chassis.forward_speed_m_s(), 0.0, "must come to rest");
    sim.chassis.position.y - start
}

/// A two-unit bus: head and trailer of 14 t each, four axles rated for 80 kN each.
fn articulated(loaded_extra_kg: f32) -> VehicleSimulation {
    let mut sim = VehicleSimulation::from_manifest(&file(14.0, 1.2, 3.0, 80.0));
    sim.chassis.total_mass_kg += loaded_extra_kg;
    let mut trailer = UnitLoad::with_axles(14_000.0 + loaded_extra_kg, 2, 0);
    for a in &mut trailer.axles {
        a.rated_n = 80_000.0;
    }
    sim.set_trailers(vec![trailer]);
    sim
}

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
fn braking_pitches_the_body_by_the_centre_of_gravity_and_the_axle_layout() {
    let pitch_in_braking = |cg: f32, half: f32| {
        let mut sim = at_speed(
            VehicleSimulation::from_manifest(&file(10.0, cg, half, 100.0)),
            KMH_50,
        );
        sim.inputs.set_service_brake(1.0);
        for _ in 0..100 {
            sim.step(0.01);
        }
        (
            sim.body.expect("body").pitch_rad,
            sim.axles[0].suspension_deflection_m,
        )
    };
    let (low, _) = pitch_in_braking(0.6, 3.0);
    let (high, front) = pitch_in_braking(1.5, 3.0);
    let (short, _) = pitch_in_braking(1.5, 1.5);
    assert!(
        low < 0.0 && high < low,
        "nose down, more with a high centre of gravity: {low} {high}"
    );
    assert!(
        short < high,
        "and more on a short wheelbase: {short} {high}"
    );
    assert!(
        front > 0.0,
        "the front spring is compressed in braking: {front}"
    );
}

#[test]
fn in_a_steady_brake_the_front_axle_carries_the_load_the_transfer_formula_gives() {
    // 12 t, centre of gravity 1.2 m, wheelbase 6 m, braked at a steady 4 m/s².
    let mut sim = VehicleSimulation::from_manifest(&file(12.0, 1.2, 3.0, 120.0));
    sim.rebuild_body();
    let static_front = sim.body.as_ref().expect("body").axle_load_n(0);
    let body = sim.body.as_mut().expect("body");
    for _ in 0..800 {
        body.step(0.01, -4.0, 0.0, &[]);
    }
    let front = body.axle_load_n(0);
    let want = 12_000.0 * 4.0 * 1.2 / 6.0;
    assert!(
        ((front - static_front) - want).abs() < 0.08 * want,
        "front gains {:.0} N, formula {want:.0} N",
        front - static_front
    );
    assert!(
        (body.total_load_n() - 12_000.0 * 9.81).abs() < 300.0,
        "the loads still carry the weight"
    );
}

#[test]
fn a_turn_rolls_the_body_outwards_and_loads_the_outer_wheels() {
    let mut sim = VehicleSimulation::from_manifest(&file(12.0, 1.2, 3.0, 120.0));
    sim.rebuild_body();
    let body = sim.body.as_mut().expect("body");
    // Turning right at 3 m/s²: the body rolls to the left (right side up, roll negative) and the
    // left wheels carry more.
    for _ in 0..600 {
        body.step(0.01, 0.0, 3.0, &[]);
    }
    assert!(body.roll_rad < 0.0, "roll {}", body.roll_rad);
    let (left, right) = (body.wheels[0].load_n, body.wheels[1].load_n);
    assert!(left > right * 1.1, "left {left:.0} N, right {right:.0} N");
}

#[test]
fn a_bump_under_one_wheel_shifts_load_to_the_diagonal_and_conserves_the_weight() {
    let mut sim = VehicleSimulation::from_manifest(&file(12.0, 1.2, 3.0, 120.0));
    sim.rebuild_body();
    let body = sim.body.as_mut().expect("body");
    let static_loads: Vec<f32> = body.wheels.iter().map(|w| w.static_load_n).collect();
    // The front left wheel stands 4 cm higher: its spring is compressed more.
    for _ in 0..600 {
        body.step(0.01, 0.0, 0.0, &[0.04, 0.0, 0.0, 0.0]);
    }
    let loads: Vec<f32> = body.wheels.iter().map(|w| w.load_n).collect();
    assert!(loads[0] > static_loads[0] * 1.05, "{loads:?}");
    assert!(
        loads[3] > static_loads[3] * 1.02,
        "the diagonal wheel gains: {loads:?}"
    );
    assert!(
        loads[1] < static_loads[1] && loads[2] < static_loads[2],
        "{loads:?}"
    );
    let total: f32 = loads.iter().sum();
    assert!((total - 12_000.0 * 9.81).abs() < 200.0, "{total}");
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
