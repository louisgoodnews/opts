//! Unit tests for `super` (`human.rs`).

use super::*;

const HUM: &str = "[model]\nm.cfg\n[seatheight]\n0.81\n[humangeom]\n0.04\n1.77\n[links]\n0.09\n0.00\n0.92\n0.09\n-0.03\n0.53\n0.02\n1.17\n0.18\n-0.05\n1.43\n0.44\n-0.04\n1.41\n-0.02\n1.55\n0.69\n-0.03\n1.43\n0.90\n-0.03\n1.43\n[walk_param]\n1.4\n70\n0.7\n1.0\n0.0\n[voice]\nM1\n[mass]\n0.07\n";

#[test]
fn pivots_mirror_and_order() {
    let p = HumanProfile::from_text(HUM).unwrap();
    let v = rest_pivots(&p);
    assert!((v[3].x + v[4].x).abs() < 1e-6, "left/right thighs mirror");
    assert!((v[0].y - 0.92).abs() < 1e-6, "hip height becomes Bevy Y");
    assert!(
        v[2].y > v[1].y && v[1].y > v[0].y,
        "neck above waist above hip"
    );
}

#[test]
fn seated_thigh_points_forward_and_shin_down() {
    let p = HumanProfile::from_text(HUM).unwrap();
    let q = pose_rotations(&p, HumanPose::Seated);
    let thigh = q[3] * Vec3::NEG_Y;
    assert!(
        (thigh - Vec3::NEG_Z).length() < 1e-5,
        "thigh forward: {thigh:?}"
    );
    let shin = (q[3] * q[5]) * Vec3::NEG_Y;
    assert!((shin - Vec3::NEG_Y).length() < 1e-5, "shin down: {shin:?}");
    let arm = q[8] * Vec3::X;
    assert!(
        arm.y < 0.0 && arm.x > 0.0,
        "right arm hangs down and out: {arm:?}"
    );
}

/// The right sole in the character's root frame for joint rotations `q` (forward kinematics
/// of hip, thigh and shin; the sole is the ground point under the knee pivot).
fn sole(p: &HumanProfile, q: &[Quat; 13]) -> Vec3 {
    let v = rest_pivots(p);
    let sole_from_knee = Vec3::new(0.0, -v[6].y, 0.0);
    v[4] + q[4] * ((v[6] - v[4]) + q[6] * sole_from_knee)
}

#[test]
fn a_fitted_seated_pose_puts_the_feet_on_the_floor_and_never_below_it() {
    // The test profile (hip 0.92, knee 0.53, [seatheight] 0.81) and an aXYZ-like one (hip
    // 0.84, knee 0.43, [seatheight] 0.75).
    let tall = HumanProfile::from_text(HUM).unwrap();
    let short = HumanProfile::from_text(
        &HUM.replace("0.81\n", "0.75\n")
            .replace("0.92\n0.09\n-0.03\n0.53", "0.84\n0.09\n-0.03\n0.43"),
    )
    .unwrap();
    assert!((seat_hip_lift(&tall) - 0.11).abs() < 1e-5);
    assert!((seat_hip_lift(&short) - 0.09).abs() < 1e-5);
    for p in [&tall, &short] {
        let hip = rest_pivots(p)[0].y;
        for h in [0.15_f32, 0.3, 0.38, 0.42, 0.45, 0.5, 0.56, 0.7, 1.0] {
            let q = seated_rotations(p, h);
            // The figure stands with its hip joint `lift` above the seat point, the floor `h`
            // below that point: in the root frame the floor is this high.
            let floor = hip - seat_hip_lift(p) - h;
            let sole = sole(p, &q);
            let a = seated_thigh_angle(p, h + seat_hip_lift(p));
            if a < THIGH_MAX_RAD {
                assert!(
                    sole.y >= floor - 1e-4,
                    "h {h}: sole {} below the floor {floor}",
                    sole.y
                );
            }
            if a > THIGH_MIN_RAD && a < THIGH_MAX_RAD {
                assert!(
                    (sole.y - floor).abs() < 1e-4,
                    "h {h}: the sole rests on the floor"
                );
            }
            let shin = (q[4] * q[6]) * Vec3::NEG_Y;
            assert!(
                (shin - Vec3::NEG_Y).length() < 1e-5,
                "the shins hang vertically"
            );
            assert!(
                sole.z < rest_pivots(p)[4].z,
                "the feet are in front of the hips"
            );
        }
        // Ordinary bus seats (0.38–0.5 m) are fitted, not clamped.
        for h in [0.38_f32, 0.5] {
            let a = seated_thigh_angle(p, h + seat_hip_lift(p));
            assert!(a > THIGH_MIN_RAD && a < THIGH_MAX_RAD, "h {h}: {a}");
        }
    }
    // A higher seat lowers the knees.
    let low = seated_thigh_angle(&tall, 0.40 + 0.11);
    let high = seated_thigh_angle(&tall, 0.55 + 0.11);
    assert!(high < low);
    // No seat height: the generic pose.
    assert_eq!(
        seated_rotations(&tall, 0.0),
        pose_rotations(&tall, HumanPose::Seated)
    );
}
