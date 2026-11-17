//! Bellows rings of an articulated joint: the yaw of a ring must follow the trailer's yaw.

use super::*;

/// Ring D of the Kajosoft O530G Facelift (`model_o530_g_1_e3_3.cfg`): pivot at the joint
/// (`origin_trans 0 -4.43 0.47`), yaw about the vertical (`origin_rot_y -90`), factor 1.0.
const RING_D: &str = "[mesh]\nC1_G\\przd_D.o3d\n\n[newanim]\norigin_trans\n0.000\n-4.43\n0.47\norigin_rot_y\n-90\nanim_rot\narticulation_0_alpha\n1.00\n\n++++++\n";

/// Where a point of the mesh (OMSI coordinates) goes when the variable is `value`.
fn moved_omsi(cfg: &str, value: f32, p: Vec3) -> Vec3 {
    let anim = steering_mesh(cfg, None);
    let m = animation_matrix(&anim.steps, |_| value);
    let b = m.transform_point3(bevy(p));
    Vec3::new(b.x, -b.z, b.y)
}

/// The published joint angle is counter-clockwise positive (a right-hand curve, where the trailer
/// lags and turns to the left of its leader). A segment turned by `alpha` (counter-clockwise)
/// has its rear end, 1 m behind the joint, at `joint + (sin α, -cos α)`: the tail swings to the
/// right. The ring with factor 1.0 must put the point there for a positive value.
#[test]
fn ring_yaw_moves_the_point_behind_the_joint_like_the_trailer_tail() {
    let joint = Vec3::new(0.0, -4.43, 0.47);
    let behind = joint + Vec3::new(0.0, -1.0, 0.0);
    let alpha = 30.0_f32.to_radians();
    let expected = joint + Vec3::new(alpha.sin(), -alpha.cos(), 0.0);
    let got = moved_omsi(RING_D, 30.0, behind);
    eprintln!("ring D at published alpha +30: got {got:?}, trailer tail {expected:?}");
    assert!(
        (got.x - expected.x).abs() < 0.02 && (got.y - expected.y).abs() < 0.02,
        "ring moves the tail the wrong way: got {got:?}, expected {expected:?}"
    );
}
