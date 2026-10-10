//! Human characters: skeleton from `[links]`, skinned meshes, poses and the head anchor.
//!
//! A `.hum` profile gives joint pivots; the skinned `.o3d` (chunk `0x54`) binds its vertices to
//! 13 canonical bones. The rest pose of the meshes is a T-pose (hands 0.7–0.9 m to the side), so
//! bind poses are pure translations to the pivots and a pose is a set of joint rotations.

use bevy::core::Name;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::system::Commands;
use bevy::hierarchy::BuildChildren;
use bevy::math::{Mat4, Quat, Vec3};
use bevy::pbr::PbrBundle;
use bevy::prelude::{Assets, SpatialBundle, Transform};
use bevy::render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::render::view::NoFrustumCulling;

use opts_core::formats::humans::{HumanProfile, EYE_BELOW_CROWN_M};

use crate::coords::omsi_to_bevy_pos;
use crate::model::ModelPart;

/// The 13 canonical bones in joint-index order.
pub const HUMAN_BONES: [&str; 13] = [
    "Hip", "Main", "Head", "OS_L", "OS_R", "US_L", "US_R", "OA_L", "OA_R", "UA_L", "UA_R",
    "Hand_L", "Hand_R",
];

/// Parent bone of each bone (`None` for the root).
const PARENT: [Option<usize>; 13] = [
    None,    // Hip
    Some(0), // Main
    Some(1), // Head
    Some(0), // OS_L
    Some(0), // OS_R
    Some(3), // US_L
    Some(4), // US_R
    Some(1), // OA_L
    Some(1), // OA_R
    Some(7), // UA_L
    Some(8), // UA_R
    Some(9), // UA_L -> Hand_L
    Some(10),
];

/// Pose of a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanPose {
    /// Upright, arms hanging at the resting flare angle.
    Standing,
    /// Hips on a seat, thighs horizontal, shins vertical.
    Seated,
}

/// Joint entities of a spawned human.
#[derive(Debug, Clone)]
pub struct HumanRig {
    /// Joint entities in [`HUMAN_BONES`] order.
    pub joints: [Entity; 13],
    /// Eye anchor, a child of the head joint; cameras attach here.
    pub head_anchor: Entity,
}

/// Component on the root entity of a human.
#[derive(Component, Debug, Clone)]
pub struct Human {
    /// The `.hum` profile the character was built from.
    pub profile: HumanProfile,
    /// Joints and head anchor.
    pub rig: HumanRig,
}

/// Marker for the eye anchor entity of the player's human.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HumanHead;

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

/// Spawns a human: root, joint hierarchy, head anchor and skinned mesh parts.
///
/// The root is returned inside the [`Human`] component. Position it with a `Transform` and call
/// [`apply_pose`] to pose it.
pub fn spawn_human(
    commands: &mut Commands,
    bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
    profile: &HumanProfile,
    parts: &[ModelPart],
    name: &str,
    pose: HumanPose,
) -> Entity {
    let pivots = rest_pivots(profile);
    let rotations = pose_rotations(profile, pose);
    let root = commands
        .spawn((SpatialBundle::default(), Name::new(name.to_string())))
        .id();

    let mut joints: Vec<Entity> = Vec::with_capacity(13);
    for (i, bone) in HUMAN_BONES.iter().enumerate() {
        let parent_pos = PARENT[i].map_or(Vec3::ZERO, |p| pivots[p]);
        let id = commands
            .spawn((
                SpatialBundle::from_transform(Transform {
                    translation: pivots[i] - parent_pos,
                    rotation: rotations[i],
                    scale: Vec3::ONE,
                }),
                Name::new((*bone).to_string()),
            ))
            .id();
        let parent = PARENT[i].map_or(root, |p| joints[p]);
        commands.entity(parent).add_child(id);
        joints.push(id);
    }

    let eye_rel = Vec3::new(
        0.0,
        (profile.height_m - EYE_BELOW_CROWN_M) - profile.links.neck.y,
        0.0,
    );
    let head_anchor = commands
        .spawn((
            SpatialBundle::from_transform(Transform::from_translation(eye_rel)),
            HumanHead,
            Name::new("HeadAnchor"),
        ))
        .id();
    commands.entity(joints[2]).add_child(head_anchor);

    let inverse: Vec<Mat4> = pivots.iter().map(|p| Mat4::from_translation(-*p)).collect();
    let ibp = bindposes.add(SkinnedMeshInverseBindposes::from(inverse));
    for (i, part) in parts.iter().enumerate() {
        let child = commands
            .spawn((
                PbrBundle {
                    mesh: part.mesh.clone(),
                    material: part.material.clone(),
                    ..Default::default()
                },
                SkinnedMesh {
                    inverse_bindposes: ibp.clone(),
                    joints: joints.clone(),
                },
                NoFrustumCulling,
                Name::new(format!("{name}_skin_{i}")),
            ))
            .id();
        commands.entity(root).add_child(child);
    }

    let rig = HumanRig {
        joints: std::array::from_fn(|i| joints.get(i).copied().unwrap_or(root)),
        head_anchor,
    };
    commands.entity(root).insert(Human {
        profile: profile.clone(),
        rig,
    });
    root
}

#[cfg(test)]
mod tests {
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
}
