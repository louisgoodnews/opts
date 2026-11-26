//! Traction, braking, grade and script forces of a tram.

use super::*;

#[test]
fn a_tram_accelerates_to_about_one_metre_per_second_squared_and_saturates() {
    let d = RailDynamics::tram(30_000.0);
    let a0 = d.acceleration(0.1, 0.0, &go());
    assert!((0.9..1.15).contains(&a0), "{a0}");
    let mut v = 0.0;
    for _ in 0..4000 {
        v = d.step(v, 0.0, &go(), 0.05);
    }
    assert!((v - d.max_speed).abs() < 1.0, "{v}");
}

#[test]
fn braking_stops_in_the_distance_physics_predicts() {
    let d = RailDynamics::tram(30_000.0);
    let (mut v, mut x) = (50.0 / 3.6, 0.0);
    let input = RailInput {
        brake: 1.0,
        ..Default::default()
    };
    while v > 0.0 {
        x += v * 0.01;
        v = d.step(v, 0.0, &input, 0.01);
    }
    // v²/(2a) with a ≈ 1.3 m/s² plus resistance: 74 m or a little less.
    assert!((60.0..76.0).contains(&x), "{x}");
}

#[test]
fn an_uphill_grade_slows_and_adhesion_limits_a_wet_start() {
    let d = RailDynamics::tram(30_000.0);
    assert!(d.acceleration(5.0, 0.05, &go()) < d.acceleration(5.0, 0.0, &go()) - 0.4);
    let wet = RailDynamics {
        adhesion: 0.08,
        ..d
    };
    assert!(wet.acceleration(0.1, 0.0, &go()) < 0.8);
}

#[test]
fn script_forces_replace_the_constants_and_a_script_brake_holds_the_train() {
    let d = RailDynamics::tram(30_000.0);
    let pull = RailInput {
        script: Some(ScriptForces {
            traction_n: 15_000.0,
            brake_n: 0.0,
        }),
        ..Default::default()
    };
    let a = d.acceleration(0.1, 0.0, &pull);
    assert!((a - (15_000.0 - d.davis_a) / 30_000.0).abs() < 0.01, "{a}");
    let held = RailInput {
        script: Some(ScriptForces {
            traction_n: 5_000.0,
            brake_n: 20_000.0,
        }),
        ..Default::default()
    };
    assert_eq!(d.step(0.0, 0.0, &held, 0.1), 0.0);
    // Adhesion still caps what a script asks for.
    let huge = RailInput {
        script: Some(ScriptForces {
            traction_n: 1.0e7,
            brake_n: 0.0,
        }),
        ..Default::default()
    };
    assert!(d.acceleration(0.1, 0.0, &huge) < 3.0);
}
