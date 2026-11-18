//! The seam residual: the distance between the trailer's rendered front coupling point and the
//! hitch `H` the trailer-front joint pins it to, as a function of the articulation yaw `alpha` and
//! of the trailer's pitch and roll (the head flat). Prints a table (run with `--nocapture`) and
//! asserts that the residual is below 1e-4 m for every yaw and for pitch, roll and both. Skipped
//! without `OMSI_PATH`.
//!
//! The trailer's pose is the simulation's own (`TrailerKinematics::step`, with `dt = 0` so the
//! heading stays as set). The placement is `segment_placement`, the function the render system
//! calls (the system itself needs an ECS world). The coupling points are the file values:
//! the head's `[coupling_back]` `(0, -4.421, 0.4)` (= `H`, see `super::COUPLING`) and the trailer's
//! `[coupling_front]` `(0, 4.116, 0.4)` (`26a_o530g_e3_3_d3_trailer.bus`), both OMSI.

use bevy::math::{DVec2, Vec3};
use opts_bevy::articulation::segment_placement;
use opts_bevy::{bevy_to_omsi_pos, omsi_to_bevy_pos};
use opts_sim::vehicle::trailer::TrailerKinematics;

use super::{load, COUPLING};

/// The trailer's `[coupling_front]` in its own OMSI frame.
const TRAILER_FRONT: Vec3 = Vec3::new(0.0, 4.116, 0.4);
/// The trailer's front axle distance is irrelevant here (`dt = 0`, no dynamics); any value works.
const AXLE_DISTANCE: f64 = 5.0;

/// The rendered coupling point (OMSI, metres) of a trailer at `alpha` (degrees, clockwise) with
/// pitch and roll (degrees) and the head at rest at the origin, heading 0, elevation 0.
fn rendered_coupling(alpha_deg: f64, pitch_deg: f64, roll_deg: f64) -> Vec3 {
    let hitch = DVec2::new(f64::from(COUPLING.x), f64::from(COUPLING.y));
    let mut t = TrailerKinematics::new(
        hitch,
        DVec2::new(f64::from(TRAILER_FRONT.x), f64::from(TRAILER_FRONT.y)),
        AXLE_DISTANCE,
        57.0,
        DVec2::ZERO,
        0.0,
    );
    // Set the yaw and let the simulation's own kinematics place the origin: with `dt = 0` the
    // heading is kept, and the origin is `hitch - front_offset(heading)`.
    t.heading = alpha_deg.to_radians();
    let st = t.step(DVec2::ZERO, 0.0, 0.0);

    // The render placement (articulation_render_system, the same function): the trailer's ground
    // height is 0 (flat, the same as the head's), and its pitch and roll are the trailer's own.
    let origin = Vec3::new(st.origin.x as f32, st.origin.y as f32, 0.0);
    let (translation, rotation) = segment_placement(
        origin,
        TRAILER_FRONT,
        -(st.heading as f32),
        (pitch_deg as f32).to_radians(),
        (roll_deg as f32).to_radians(),
    );
    bevy_to_omsi_pos(translation + rotation * omsi_to_bevy_pos(TRAILER_FRONT))
}

/// The residual vector `a - H` in OMSI, where `a` is the rendered coupling and `H` the hitch the
/// joint pins it to (the joint fixes `H` for every angle, so `b = H`).
fn residual(alpha_deg: f64, pitch_deg: f64, roll_deg: f64) -> Vec3 {
    rendered_coupling(alpha_deg, pitch_deg, roll_deg) - COUPLING
}

#[test]
fn seam_residual_under_pitch_and_roll() {
    let Some(_) = load() else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    println!("alpha(deg)  pitch(deg)  roll(deg)  |a-H|(m)     a-H (OMSI x, y, z) (m)");
    for alpha in [0.0, 12.0, -12.0, 25.0, -25.0] {
        for (pitch, roll) in [(0.0, 0.0), (3.0, 0.0), (0.0, -2.0), (3.0, -2.0)] {
            let r = residual(alpha, pitch, roll);
            println!(
                "{alpha:>10.1} {pitch:>10.1} {roll:>10.1}  {:>10.6}  ({:+.6}, {:+.6}, {:+.6})",
                r.length(),
                r.x,
                r.y,
                r.z
            );
            assert!(
                r.length() < 1e-4,
                "alpha {alpha}, pitch {pitch}, roll {roll}: residual {r:?} from the hitch"
            );
        }
    }
    // Slope of the pitch-only residual at alpha 0, and the pitch that gives 2 cm and 10 cm.
    println!("pitch-only sweep (alpha 0, roll 0):");
    for pitch in [0.1, 0.25, 0.5, 1.0, 2.0, 3.0] {
        let r = residual(0.0, pitch, 0.0);
        println!("  pitch {pitch:>5.2} deg -> |a-H| {:.6} m", r.length());
    }
    // The direction of the residual at 0.3 % grade (0.17 deg), for the in-game note.
    let grade = (0.003_f64).atan().to_degrees();
    let r = residual(0.0, grade, 0.0);
    println!(
        "0.3 % grade ({grade:.4} deg): |a-H| {:.6} m, vector ({:+.6}, {:+.6}, {:+.6})",
        r.length(),
        r.x,
        r.y,
        r.z
    );
}
