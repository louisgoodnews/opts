//! The trailer end follows the trailer's rigid pose about the hitch, and the head end does not
//! move, at the yaws of the check (pitch 3 and roll -2 degrees as well).

use bevy::math::{DMat4, DVec3, Mat4, Vec3};
use opts_bevy::articulation::joint_model_transform;
use opts_bevy::mesh::remap_trailer_end;
use opts_sim::vehicle::trailer_front::joint_transform;

use super::{identity_mats, load, posed, COUPLING, RING_D, TRAILER_JOINT};

const ALPHAS_DEG: [f64; 4] = [12.0, -12.0, 25.0, -25.0];

/// The OMSI joint of the trailer at clockwise yaw `alpha` (radians), with the fixed pitch and roll
/// of the check, and its matrix in the model space.
fn rigid(alpha: f64) -> (DMat4, Mat4) {
    let hitch = DVec3::new(
        f64::from(COUPLING.x),
        f64::from(COUPLING.y),
        f64::from(COUPLING.z),
    );
    let j = joint_transform(hitch, alpha, 3f64.to_radians(), (-2f64).to_radians());
    let m = joint_model_transform(j).to_matrix();
    (j, m)
}

/// The rigid pose of an OMSI point, mapped back to the model space `(x, z, -y)`.
fn rigid_model(j: &DMat4, omsi: Vec3) -> Vec3 {
    let q = j.transform_point3(DVec3::new(
        f64::from(omsi.x),
        f64::from(omsi.y),
        f64::from(omsi.z),
    ));
    Vec3::new(q.x as f32, q.z as f32, -(q.y as f32))
}

#[test]
fn kajosoft_trailer_end_follows_the_rigid_trailer_pose() {
    let Some(b) = load() else {
        return;
    };
    let (out, _) = remap_trailer_end(&b.table, &b.band, RING_D, TRAILER_JOINT);
    for alpha_deg in ALPHAS_DEG {
        let (j, m) = rigid(alpha_deg.to_radians());
        let mut mats = identity_mats();
        mats[usize::from(TRAILER_JOINT)] = m;
        let mut pure = 0;
        for &i in &b.band {
            let f: f32 = out[i]
                .0
                .iter()
                .zip(&out[i].1)
                .filter(|(joint, _)| **joint == TRAILER_JOINT)
                .map(|(_, w)| *w)
                .sum();
            let p = b.model[i];
            let expected = rigid_model(&j, b.omsi[i]);
            let got = posed(&out[i], &mats, p);
            let blend = expected * f + p * (1.0 - f);
            assert!(
                got.abs_diff_eq(blend, 1e-4),
                "alpha {alpha_deg}: {got:?} != {blend:?}"
            );
            if f >= 1.0 - 1e-6 {
                pure += 1;
                assert!(got.abs_diff_eq(expected, 1e-4), "alpha {alpha_deg}");
            }
        }
        assert!(pure > 0, "some trailer-end vertices carry the joint alone");
    }
}

#[test]
fn kajosoft_head_end_does_not_move() {
    let Some(b) = load() else {
        return;
    };
    let (out, _) = remap_trailer_end(&b.table, &b.band, RING_D, TRAILER_JOINT);
    for alpha_deg in ALPHAS_DEG {
        let (_, m) = rigid(alpha_deg.to_radians());
        let mut mats = identity_mats();
        mats[usize::from(TRAILER_JOINT)] = m;
        for (i, p) in b.model.iter().enumerate() {
            if b.band.contains(&i) {
                continue;
            }
            assert_eq!(out[i], b.table[i], "head vertex {i} keeps its skin");
            assert!(posed(&out[i], &mats, *p).abs_diff_eq(*p, 1e-6));
        }
    }
}
