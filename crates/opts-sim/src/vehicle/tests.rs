//! Unit tests of `mod.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;
use crate::vehicle::inputs::GearPosition;

#[test]
fn test_simulation_acceleration() {
    let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_gear(GearPosition::Drive);
    sim.inputs.set_throttle(1.0);

    // Run for 2 seconds (120 steps at 60 Hz)
    for _ in 0..120 {
        sim.step(1.0 / 60.0);
    }

    // Bus should have moved forward and accelerated
    assert!(sim.chassis.forward_speed_km_h() > 10.0);
    assert!(sim.chassis.position.y > 0.0);
}

#[test]
fn test_simulation_determinism() {
    let mut sim1 = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim1.inputs.set_parking_brake(false);
    sim1.inputs.set_gear(GearPosition::Drive);
    sim1.inputs.set_throttle(0.8);
    sim1.inputs.set_steering(0.3);

    let mut sim2 = sim1.clone();

    for _ in 0..60 {
        sim1.step(1.0 / 60.0);
        sim2.step(1.0 / 60.0);
    }

    // Must produce bit-for-bit identical results
    assert_eq!(sim1.chassis.position, sim2.chassis.position);
    assert_eq!(sim1.chassis.local_velocity, sim2.chassis.local_velocity);
    assert_eq!(sim1.chassis.heading_rad, sim2.chassis.heading_rad);
}

/// A bus with one unit behind it: hitch 1.5 m behind the chassis origin, front coupling
/// 1 m ahead of the unit's origin, rear axle 6 m behind that.
fn articulated() -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    sim.attach_trailer(DVec2::new(0.0, -1.5), DVec2::new(0.0, 1.0), 7.0, 52.5, None);
    sim.inputs.set_parking_brake(false);
    sim.inputs.set_gear(GearPosition::Drive);
    sim.inputs.set_throttle(0.4);
    sim.inputs.set_steering(0.5);
    sim
}

#[test]
fn the_consist_is_stepped_with_the_chassis_and_stays_hitched() {
    let mut sim = articulated();
    for _ in 0..600 {
        sim.step(1.0 / 64.0);
    }
    let alpha = sim.consist_alpha_rad(0).unwrap();
    assert!(alpha.abs() > 0.05 && alpha.abs() <= 52.5_f64.to_radians());
    let unit = &sim.consist[0];
    let (pos, heading) = sim.chassis_pose();
    let hitch = unit.hitch(pos, heading);
    let front = unit.origin
        + DVec2::new(unit.heading.cos(), -unit.heading.sin()) * unit.front_local.x
        + DVec2::new(unit.heading.sin(), unit.heading.cos()) * unit.front_local.y;
    assert!((front - hitch).length() < 1e-6);
    assert_eq!(sim.consist_alpha_rad(1), None);
}

/// Design V4: a consist driven into a circle, serialised, restored and driven on continues
/// exactly like the unbroken run (the acceptance asks for 1 mm and 0.01 degrees).
#[test]
fn a_serialised_consist_resumes_like_the_unbroken_run() {
    let mut a = articulated();
    for _ in 0..400 {
        a.step(1.0 / 64.0);
    }
    let bytes = bincode::serialize(&a).unwrap();
    let mut b: VehicleSimulation = bincode::deserialize(&bytes).unwrap();
    assert_eq!(a, b, "the restored state equals the saved one");
    for _ in 0..200 {
        a.step(1.0 / 64.0);
        b.step(1.0 / 64.0);
    }
    assert_eq!(a, b);
    let d = (a.consist[0].origin - b.consist[0].origin).length();
    let dalpha = (a.consist_alpha_rad(0).unwrap() - b.consist_alpha_rad(0).unwrap()).abs();
    assert!(d < 1e-3 && dalpha.to_degrees() < 0.01);
    assert!(
        a.consist_alpha_rad(0).unwrap().abs() > 0.05,
        "it is in a curve"
    );
}

#[test]
fn a_state_saved_without_a_consist_still_loads() {
    // `consist` and `teleports` are `serde(default)`; a vehicle of that shape round-trips.
    let solo = VehicleSimulation::default();
    let back: VehicleSimulation =
        bincode::deserialize(&bincode::serialize(&solo).unwrap()).unwrap();
    assert!(back.consist.is_empty());
    assert_eq!(solo, back);
}

#[test]
fn teleport_hangs_the_consist_straight_with_no_velocity() {
    let mut sim = articulated();
    for _ in 0..400 {
        sim.step(1.0 / 64.0);
    }
    assert!(sim.consist_alpha_rad(0).unwrap().abs() > 0.05);
    sim.teleport(Vec2::new(500.0, -300.0), 1.2);
    assert_eq!(sim.teleports, 1);
    assert!(sim.consist_alpha_rad(0).unwrap().abs() < 1e-12);
    // Standing still after the jump: the unit does not move.
    sim.inputs.set_throttle(0.0);
    sim.inputs.set_steering(0.0);
    sim.chassis.local_velocity = Vec2::ZERO;
    sim.chassis.yaw_rate_rad_s = 0.0;
    let before = sim.consist[0].origin;
    sim.step(1.0 / 64.0);
    assert!((sim.consist[0].origin - before).length() < 1e-6);
    assert!(sim.consist_alpha_rad(0).unwrap().abs() < 1e-6);
}

fn unit(mass_kg: f32, axles: &[(f32, bool)]) -> UnitLoad {
    UnitLoad {
        mass_kg,
        axles: axles
            .iter()
            .map(|&(y, is_driven)| UnitAxle {
                longitudinal_m: y,
                is_driven,
                rated_n: consist::DEFAULT_RATED_AXLE_N,
            })
            .collect(),
    }
}

/// A bus of 11.5 t with a trailer of `trailer_kg` (none: solo), on a straight line.
fn towing(trailer_kg: Option<f32>, trailer_driven: bool) -> VehicleSimulation {
    let mut sim = VehicleSimulation::new_standard_bus(11_500.0, 5.85);
    if let Some(m) = trailer_kg {
        let load = unit(m, &[(-4.0, trailer_driven)]);
        sim.attach_trailer(
            DVec2::new(0.0, -2.0),
            DVec2::new(0.0, 1.0),
            6.0,
            52.5,
            Some(load),
        );
    }
    sim.inputs.set_parking_brake(false);
    sim
}

/// Design V5 / acceptance 2: with the same wheel torque the acceleration scales with
/// `1 / Σm` within 2 %. The traction part is the difference to a run without torque from
/// the same state, so resistances cancel.
#[test]
fn script_torque_accelerates_the_whole_consist_mass() {
    let accel = |trailer: Option<f32>, torque: f32| {
        let mut sim = towing(trailer, false);
        sim.chassis.local_velocity.y = 5.0;
        sim.script_drive = Some(ScriptDrive {
            wheel_torque_nm: torque,
            brake_force_n: 0.0,
        });
        sim.step(1.0 / 64.0);
        sim.chassis.local_acceleration.y
    };
    let traction = |trailer| accel(trailer, 4_000.0) - accel(trailer, 0.0);
    let (solo, pair) = (traction(None), traction(Some(10_000.0)));
    let expected = solo * 11_500.0 / 21_500.0;
    assert!(
        (pair - expected).abs() < 0.02 * expected,
        "traction accel {pair} vs {expected}"
    );
    assert!((towing(Some(10_000.0), false).consist_mass_kg() - 21_500.0).abs() < 1e-3);
}

/// Plan step 4: stopping distance of head-only versus consist differs by the mass ratio
/// (here: not at all, since brake force is limited by friction, `μ·g` per kilogram).
#[test]
fn brake_limit_follows_the_summed_axle_loads() {
    let stop = |sim: &mut VehicleSimulation| {
        sim.chassis.local_velocity.y = 14.0; // ~50 km/h
        sim.script_drive = Some(ScriptDrive {
            wheel_torque_nm: 0.0,
            brake_force_n: 1.0e9, // more than the tyres can carry
        });
        sim.inputs.set_service_brake(1.0);
        let mut s = 0.0_f32;
        for _ in 0..2000 {
            let before = sim.chassis.position.y;
            sim.step(1.0 / 64.0);
            s += sim.chassis.position.y - before;
            if sim.chassis.forward_speed_m_s() <= 0.0 {
                break;
            }
        }
        s
    };
    let (solo, pair) = (
        stop(&mut towing(None, false)),
        stop(&mut towing(Some(10_000.0), false)),
    );
    // Same μ·g deceleration: the distance matches within 2 % (every axle brakes).
    assert!((solo - pair).abs() < 0.02 * solo, "{solo} vs {pair}");
    let ideal = 14.0_f32 * 14.0 / (2.0 * ROAD_FRICTION * GRAVITY);
    assert!(solo > ideal * 0.95, "{solo} vs ideal {ideal}");
}

/// Plan step 5: the drive cap follows the driven axles of all units.
#[test]
fn drive_limit_follows_the_driven_axles_of_all_units() {
    let head_only = towing(Some(10_000.0), false);
    let all_driven = towing(Some(10_000.0), true);
    let (d0, b0) = head_only.friction_loads_n();
    let (d1, b1) = all_driven.friction_loads_n();
    assert!((b0 - b1).abs() < 1.0, "braked load is the same");
    assert!((b0 - 21_500.0 * GRAVITY).abs() < 5.0);
    assert!((d1 - d0 - 10_000.0 * GRAVITY).abs() < 5.0, "{d0} -> {d1}");
    // The head's rear axle carries half its weight on this symmetric bus.
    assert!((d0 - 0.5 * 11_500.0 * GRAVITY).abs() < 5.0);
}

/// Plan step 6: on a straight run `F_h = m_last·a + resistances`.
#[test]
fn hitch_force_of_the_last_unit_is_mass_times_acceleration_plus_rolling() {
    let mut sim = towing(Some(10_000.0), false);
    sim.script_drive = Some(ScriptDrive {
        wheel_torque_nm: 6_000.0,
        brake_force_n: 0.0,
    });
    for _ in 0..64 {
        sim.step(1.0 / 64.0);
    }
    let a = sim.chassis.local_acceleration.y;
    let f = sim.hitch_force_n(0).unwrap();
    let want = 10_000.0 * (a + drivetrain::ROLLING_RESISTANCE_COEFF * GRAVITY);
    assert!(a > 0.1 && (f - want).abs() < 1.0, "{f} vs {want}");
    assert_eq!(sim.hitch_force_n(1), None);
}

#[test]
fn loads_survive_serialisation() {
    let sim = towing(Some(9_000.0), true);
    let back: VehicleSimulation = bincode::deserialize(&bincode::serialize(&sim).unwrap()).unwrap();
    assert_eq!(sim, back);
}

/// Samples a plane `z = slope · y` under every probe.
fn samples_on_slope(sim: &VehicleSimulation, slope: f32) -> Vec<Option<GroundSample>> {
    sim.ground_probes()
        .into_iter()
        .map(|probe| {
            let h = probe.points().map(|p| p.y * slope);
            Some(GroundSample::from_heights(probe, h))
        })
        .collect()
}

/// Acceptance 3 and the NG272 target in the harness: on flat ground pitch and roll stay 0;
/// on a constant grade every unit takes the grade angle and `β` is 0, and a unit that stands
/// on a different grade shows the difference of the two angles.
#[test]
fn fixed_step_ground_following_gives_zero_on_flat_and_the_grade_difference_on_a_step() {
    let mut sim = articulated();
    sim.inputs.set_throttle(0.0);
    sim.apply_ground(&samples_on_slope(&sim, 0.0), 1.0 / 64.0);
    let (beta, gamma) = sim.joint_angles_deg(0).unwrap();
    assert!(beta.abs() < 0.01 && gamma.abs() < 0.01);
    for _ in 0..200 {
        let s = samples_on_slope(&sim, 0.1);
        sim.apply_ground(&s, 1.0 / 64.0);
    }
    let grade_deg = 0.1_f32.atan().to_degrees();
    let (beta, _) = sim.joint_angles_deg(0).unwrap();
    // Chassis and unit both settle on the grade: β returns to 0 (the head was sampled on
    // flat ground first, so it has to catch up; after 3 s both agree).
    assert!(beta.abs() < 0.05, "{beta}");
    assert!((sim.consist_body(0).unwrap().pitch.to_degrees() - grade_deg).abs() < 0.05);
    // Unit on the grade, chassis still level: β is minus the grade angle (nose-up follower).
    let mut flat_head = articulated();
    let mut s = samples_on_slope(&flat_head, 0.0);
    s[1] = samples_on_slope(&flat_head, 0.1)[1];
    flat_head.apply_ground(&s, 1.0 / 64.0);
    let (beta, _) = flat_head.joint_angles_deg(0).unwrap();
    assert!((beta + grade_deg).abs() < 0.2, "{beta} vs {grade_deg}");
}

#[test]
fn ground_state_is_reseeded_by_teleport_and_serialised() {
    let mut sim = articulated();
    sim.apply_ground(&samples_on_slope(&sim, 0.05), 1.0 / 64.0);
    let back: VehicleSimulation = bincode::deserialize(&bincode::serialize(&sim).unwrap()).unwrap();
    assert_eq!(sim, back);
    assert!(sim.joint_angles_deg(0).is_some());
    sim.teleport(Vec2::new(10.0, 10.0), 0.0);
    assert!(
        sim.joint_angles_deg(0).is_none(),
        "no body until the next sample"
    );
    // A missing sample keeps the state.
    sim.apply_ground(&samples_on_slope(&sim, 0.05), 1.0 / 64.0);
    let kept = sim.consist_body(0);
    sim.apply_ground(&[None, None], 1.0 / 64.0);
    assert_eq!(sim.consist_body(0), kept);
}

#[test]
fn a_head_with_no_driven_axle_counts_as_driven_on_all() {
    let mut sim = VehicleSimulation::new_standard_bus(10_000.0, 5.0);
    sim.axles
        .iter_mut()
        .for_each(|a| a.config.is_driven = false);
    let (driven, braked) = sim.friction_loads_n();
    assert!((driven - braked).abs() < 1.0);
    sim.set_trailers(vec![UnitLoad::with_axles(5_000.0, 2, 2)]);
    assert!((sim.driven_mass_kg() - 15_000.0).abs() < 1.0);
}

/// A forward torque while the vehicle rolls backwards slows the roll: the traction part of the
/// acceleration (the difference to a run without torque) is forward at any speed, also negative.
#[test]
fn forward_script_torque_pushes_forward_while_rolling_backwards() {
    for v in [-0.5_f32, -5.0, -12.0] {
        let accel = |torque: f32| {
            let mut sim = towing(None, false);
            sim.chassis.local_velocity.y = v;
            sim.script_drive = Some(ScriptDrive {
                wheel_torque_nm: torque,
                brake_force_n: 0.0,
            });
            for _ in 0..4 {
                sim.step(1.0 / 64.0);
            }
            sim.chassis.local_acceleration.y
        };
        let traction = accel(4_000.0) - accel(0.0);
        assert!(traction > 0.0, "at {v} m/s the torque pushed {traction}");
    }
}

/// Rolling backwards with a constant forward idle torque (a bus in drive on a flat road): the
/// speed moves towards zero, it never grows backwards.
#[test]
fn a_steady_forward_torque_slows_a_roll_back_on_the_flat() {
    let mut sim = towing(None, false);
    sim.chassis.local_velocity.y = -4.0;
    sim.script_drive = Some(ScriptDrive {
        wheel_torque_nm: 4_000.0,
        brake_force_n: 0.0,
    });
    let mut last = sim.chassis.forward_speed_m_s();
    for i in 0..640 {
        sim.step(1.0 / 64.0);
        let v = sim.chassis.forward_speed_m_s();
        assert!(v >= last - 1e-3, "step {i}: {last} -> {v}");
        last = v;
    }
    assert!(last > -4.0, "ended at {last}");
}
