//! Consists, masses and the braked and powered mass.

use super::*;

#[test]
fn a_single_body_consist_is_the_old_tram() {
    let a = RailDynamics::for_consist(ConsistLoad::single(30_000.0));
    let b = RailDynamics::tram(30_000.0);
    assert_eq!(a, b);
    assert_eq!(a.powered_mass_kg, 30_000.0);
    assert_eq!(a.braked_mass_kg, 30_000.0);
}


#[test]
fn a_heavy_consist_accelerates_with_the_traction_of_its_powered_cars_only() {
    let (train, loco_alone) = freight();
    let a_train = train.acceleration(0.1, 0.0, &go());
    let a_loco = loco_alone.acceleration(0.1, 0.0, &go());
    // The pull is the locomotive's 1.1 N/kg x 84 t in both; only the mass differs.
    let f = 1.1 * 84_000.0;
    assert!((pull_n(&train, a_train, 0.1) / f - 1.0).abs() < 0.01);
    assert!((pull_n(&loco_alone, a_loco, 0.1) / f - 1.0).abs() < 0.01);
    // 884 t with rolling resistance (26 kN): about 0.075 m/s², a fraction of the lone loco's 1.1.
    assert!((0.06..0.09).contains(&a_train), "{a_train}");
    assert!(a_loco / a_train > 10.0, "{a_loco} / {a_train}");
}

#[test]
fn acceleration_scales_with_one_over_total_mass_at_equal_traction() {
    let build = |total| {
        RailDynamics::for_consist(ConsistLoad {
            total_kg: total,
            powered_kg: 100_000.0,
            braked_kg: total,
        })
    };
    let (light, heavy) = (build(200_000.0), build(400_000.0));
    assert_eq!(light.max_traction_n, heavy.max_traction_n);
    let (a_l, a_h) = (
        light.acceleration(0.1, 0.0, &go()),
        heavy.acceleration(0.1, 0.0, &go()),
    );
    // Equal force on twice the mass: the force that accelerated them is the same within 2 %.
    let (f_l, f_h) = (pull_n(&light, a_l, 0.1), pull_n(&heavy, a_h, 0.1));
    assert!((f_l / f_h - 1.0).abs() < 0.02, "{f_l} vs {f_h}");
    assert!(a_l > 1.7 * a_h && a_l < 2.6 * a_h, "{a_l} vs {a_h}");
}

#[test]
fn a_gradient_pulls_on_the_whole_train_but_traction_does_not_grow() {
    let (train, _) = freight();
    // 1 % uphill: 884 t x g x 0.01 = 87 kN, nearly all of the locomotive's 92 kN.
    let on_grade = train.acceleration(5.0, 0.01, &go());
    let flat = train.acceleration(5.0, 0.0, &go());
    assert!(on_grade < flat - 0.08, "{on_grade} vs {flat}");
}

#[test]
fn braking_force_follows_the_braked_mass() {
    let all = RailDynamics::for_consist(ConsistLoad {
        total_kg: 400_000.0,
        powered_kg: 100_000.0,
        braked_kg: 400_000.0,
    });
    let head_only = RailDynamics::for_consist(ConsistLoad {
        total_kg: 400_000.0,
        powered_kg: 100_000.0,
        braked_kg: 100_000.0,
    });
    let brake = RailInput {
        brake: 1.0,
        ..Default::default()
    };
    let (a_all, a_head) = (
        all.acceleration(10.0, 0.0, &brake),
        head_only.acceleration(10.0, 0.0, &brake),
    );
    assert!(a_all < -1.2, "{a_all}");
    assert!((a_head / a_all - 0.25).abs() < 0.06, "{a_head} / {a_all}");
}

#[test]
fn masses_are_clamped_into_a_sensible_order() {
    let d = RailDynamics::for_consist(ConsistLoad {
        total_kg: 10_000.0,
        powered_kg: 90_000.0,
        braked_kg: 1.0,
    });
    assert_eq!(d.mass_kg, 10_000.0);
    assert_eq!(d.powered_mass_kg, 10_000.0);
    assert_eq!(d.braked_mass_kg, 5_000.0);
}

#[test]
fn resistance_and_brakes_never_make_the_speed_negative() {
    let d = RailDynamics::tram(30_000.0);
    let v = d.step(
        0.05,
        0.0,
        &RailInput {
            emergency: true,
            ..Default::default()
        },
        0.5,
    );
    assert_eq!(v, 0.0);
}
