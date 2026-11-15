//! Poses: the rest pivots of the joints, the joint rotations of a pose and applying them.

use super::*;

/// Pivot of each bone in Bevy space (rest pose, character origin on the ground).
///
/// `[links]` are in OMSI's character frame (x lateral, y longitudinal, z vertical) and list the
/// right side; the left side mirrors across `x = 0`.
#[must_use]
pub fn rest_pivots(p: &HumanProfile) -> [Vec3; 13] {
    let l = &p.links;
    let at = |x: f32, y: f32, z: f32| omsi_to_bevy_pos(Vec3::new(x, y, z));
    [
        at(0.0, l.hip.y, l.hip.z),                     // Hip
        at(0.0, l.waist.x, l.waist.y),                 // Main
        at(0.0, l.neck.x, l.neck.y),                   // Head
        at(-l.hip.x, l.hip.y, l.hip.z),                // OS_L
        at(l.hip.x, l.hip.y, l.hip.z),                 // OS_R
        at(-l.knee.x, l.knee.y, l.knee.z),             // US_L
        at(l.knee.x, l.knee.y, l.knee.z),              // US_R
        at(-l.shoulder.x, l.shoulder.y, l.shoulder.z), // OA_L
        at(l.shoulder.x, l.shoulder.y, l.shoulder.z),  // OA_R
        at(-l.elbow.x, l.elbow.y, l.elbow.z),          // UA_L
        at(l.elbow.x, l.elbow.y, l.elbow.z),           // UA_R
        at(-l.hand.x, l.hand.y, l.hand.z),             // Hand_L
        at(l.hand.x, l.hand.y, l.hand.z),              // Hand_R
    ]
}

/// Local joint rotations for a pose (identity = T-pose rest).
#[must_use]
pub fn pose_rotations(p: &HumanProfile, pose: HumanPose) -> [Quat; 13] {
    let mut q = [Quat::IDENTITY; 13];
    // Arms: rotate down from the horizontal rest by the resting flare angle (`upper_arm_beta`).
    let beta = p.walk.upper_arm_beta_deg.to_radians();
    q[7] = Quat::from_rotation_z(beta); // OA_L (-x): +beta swings it downwards
    q[8] = Quat::from_rotation_z(-beta); // OA_R (+x)
    if pose == HumanPose::Seated {
        let quarter = std::f32::consts::FRAC_PI_2;
        q[3] = Quat::from_rotation_x(quarter); // OS_L: thigh forward
        q[4] = Quat::from_rotation_x(quarter); // OS_R
        q[5] = Quat::from_rotation_x(-quarter); // US_L: shin back down
        q[6] = Quat::from_rotation_x(-quarter); // US_R
    }
    q
}

/// Applies `pose` to the joint transforms of a spawned human.
pub fn apply_pose(
    transforms: &mut bevy::prelude::Query<&mut Transform>,
    human: &Human,
    pose: HumanPose,
) {
    let rotations = pose_rotations(&human.profile, pose);
    for (i, joint) in human.rig.joints.iter().enumerate() {
        if let Ok(mut t) = transforms.get_mut(*joint) {
            t.rotation = rotations[i];
        }
    }
}

/// World offset of the hip pivot from the character origin in the rest pose, so a seated
/// character can be placed with its hips on the seat contact point.
#[must_use]
pub fn hip_offset(p: &HumanProfile) -> Vec3 {
    rest_pivots(p)[0]
}
