//! Human characters: skeleton from `[links]`, skinned meshes, poses and the head anchor.
//!
//! A `.hum` profile gives joint pivots; the skinned `.o3d` (chunk `0x54`) binds its vertices to
//! 13 canonical bones. The rest pose of the meshes is a T-pose (hands 0.7–0.9 m to the side), so
//! bind poses are pure translations to the pivots and a pose is a set of joint rotations.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
use bevy::ecs::system::Commands;
use bevy::math::{Mat4, Quat, Vec3};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::Mesh3d;
use bevy::pbr::MeshMaterial3d;
use bevy::prelude::{Assets, Transform};

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

/// Marker for the eye anchor entity of every human (the player's and the pedestrians').
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct HumanHead;

/// Marker for the eye anchor of the **player's** human: the ego camera attaches here.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PlayerHead;

/// Marker for the eye anchor of the human that sits in **a** session vehicle's driver seat (the
/// active one or a parked one). [`sync_player_head`] keeps [`PlayerHead`] on the one whose vehicle
/// is the [`crate::vehicle_sim::PlayerVehicle`], so the ego and cockpit cameras always find exactly
/// one head.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DriverHead;

/// On the root of the player's human (a child of the vehicle): once its pose is in place, the
/// character is shifted so that its eyes lie at `target` (vehicle-local Bevy space), where the
/// vehicle's driver camera is. The camera and the head then coincide, in the cockpit view and in
/// the ego view alike.
#[derive(Component, Debug, Clone, Copy)]
pub struct AlignEyes {
    /// The eye position in the vehicle's frame.
    pub target: Vec3,
    /// Frames to wait for the first transform propagation of the new hierarchy.
    pub wait: u8,
}

/// Marks the player's head as the eye position once the hierarchy is propagated (see [`AlignEyes`]).
pub fn align_driver_eyes(
    mut commands: bevy::prelude::Commands,
    mut humans: bevy::prelude::Query<(Entity, &Human, &mut AlignEyes)>,
    globals: bevy::prelude::Query<&bevy::prelude::GlobalTransform>,
) {
    for (e, human, mut align) in &mut humans {
        if align.wait > 0 {
            align.wait -= 1;
            continue;
        }
        if globals.get(human.rig.head_anchor).is_err() {
            continue;
        }
        // The human stays on the seat (`[drivpos]`); the cockpit and ego cameras go to its eyes.
        commands.entity(e).remove::<AlignEyes>();
        commands.entity(human.rig.head_anchor).insert(DriverHead);
    }
}

/// Puts [`PlayerHead`] on the [`DriverHead`] whose vehicle is the active one and nowhere else.
pub fn sync_player_head(
    mut commands: bevy::prelude::Commands,
    heads: bevy::prelude::Query<
        (Entity, bevy::prelude::Has<PlayerHead>),
        bevy::prelude::With<DriverHead>,
    >,
    parents: bevy::prelude::Query<&bevy::prelude::ChildOf>,
    players: bevy::prelude::Query<(), bevy::prelude::With<crate::vehicle_sim::PlayerVehicle>>,
) {
    for (head, has) in &heads {
        let active = players.contains(parents.root_ancestor(head));
        if active && !has {
            commands.entity(head).insert(PlayerHead);
        } else if !active && has {
            commands.entity(head).remove::<PlayerHead>();
        }
    }
}

/// Hides the player's own character while the camera sits at the driver's seat (cockpit and ego
/// views), so the eye never looks through the head or the torso; shows it in every other view.
///
/// The character is the player's when it belongs to the player's vehicle (it is a child of its
/// root): that holds from the frame it exists, whereas [`PlayerHead`] is only set a few frames
/// later (eye alignment, [`sync_player_head`]), which showed the back of the head in the first
/// seconds of a drive in the driver's view.
pub fn hide_driver_in_seat_views(
    state: bevy::prelude::Res<crate::camera::CameraState>,
    humans: bevy::prelude::Query<
        (Entity, &mut bevy::prelude::Visibility),
        bevy::prelude::With<Human>,
    >,
    parents: bevy::prelude::Query<&bevy::prelude::ChildOf>,
    players: bevy::prelude::Query<(), bevy::prelude::With<crate::vehicle_sim::PlayerVehicle>>,
) {
    let seat = matches!(
        state.mode,
        crate::camera::CameraMode::Cockpit | crate::camera::CameraMode::Ego
    );
    let mut humans = humans;
    for (human, mut vis) in &mut humans {
        if !players.contains(parents.root_ancestor(human)) {
            continue;
        }
        let want = if seat {
            bevy::prelude::Visibility::Hidden
        } else {
            bevy::prelude::Visibility::Inherited
        };
        if *vis != want {
            *vis = want;
        }
    }
}

/// How far the eyes (the nose bridge) lie in front of the head's centre line (m).
pub const EYES_FORWARD_M: f32 = 0.09;

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

/// The most the hip joint is put above the point a seated character rests on (m); the corpus
/// lies between 0 and 0.14.
const MAX_SEAT_LIFT_M: f32 = 0.15;
/// The thigh angle of a fitted seated pose, from hanging straight down (rad): from sloping down
/// by 40° on a high seat (the feet then hang) to the knees 25° up on a low one.
const THIGH_MIN_RAD: f32 = 50.0 * std::f32::consts::PI / 180.0;
const THIGH_MAX_RAD: f32 = 115.0 * std::f32::consts::PI / 180.0;

/// How far the hip joint of a seated character lies above the point it rests on (m).
///
/// `[seatheight]` is the height, in the standing character's frame, of the part that rests on a
/// seat: in all 58 installed `.hum` files it lies 0–0.14 m below the hip joint (`[links]`), the
/// thickness of the thigh and buttocks under the hip joint. A `[passpos]` seat point is that
/// contact ("Arsch"), so the hip joint goes this much above it.
#[must_use]
pub fn seat_hip_lift(p: &HumanProfile) -> f32 {
    (p.links.hip.z - p.seat_height_m).clamp(0.0, MAX_SEAT_LIFT_M)
}

/// The thigh angle from hanging straight down (rad) at which the shin, hanging vertically from
/// the knee, puts the sole on the floor `hip_above_floor_m` below the hip joint; clamped to a
/// natural range (a high seat leaves the feet hanging, a low one raises the knees).
///
/// The thigh is the hip-to-knee vector in the sagittal plane of `[links]`, the shin and foot the
/// knee's height above the ground.
#[must_use]
pub fn seated_thigh_angle(p: &HumanProfile, hip_above_floor_m: f32) -> f32 {
    let dy = (p.links.hip.z - p.links.knee.z).max(0.05);
    let dz = p.links.hip.y - p.links.knee.y;
    let thigh = dy.hypot(dz);
    let shin = p.links.knee.z.max(0.05);
    (dz.atan2(dy) + ((hip_above_floor_m - shin) / thigh).clamp(-1.0, 1.0).acos())
        .clamp(THIGH_MIN_RAD, THIGH_MAX_RAD)
}

/// The seated pose fitted to a seat `seat_height_m` above the floor (the `h` of `[passpos]`):
/// thighs at [`seated_thigh_angle`], shins vertical, so that the feet rest on the floor. A seat
/// of unknown height (`0`) keeps the generic pose (thighs horizontal).
#[must_use]
pub fn seated_rotations(p: &HumanProfile, seat_height_m: f32) -> [Quat; 13] {
    let mut q = pose_rotations(p, HumanPose::Seated);
    if seat_height_m > 0.0 {
        let a = seated_thigh_angle(p, seat_height_m + seat_hip_lift(p));
        q[3] = Quat::from_rotation_x(a);
        q[4] = Quat::from_rotation_x(a);
        q[5] = Quat::from_rotation_x(-a);
        q[6] = Quat::from_rotation_x(-a);
    }
    q
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
        .spawn((
            Transform::default(),
            bevy::prelude::Visibility::default(),
            Name::new(name.to_string()),
        ))
        .id();

    let mut joints: Vec<Entity> = Vec::with_capacity(13);
    for (i, bone) in HUMAN_BONES.iter().enumerate() {
        let parent_pos = PARENT[i].map_or(Vec3::ZERO, |p| pivots[p]);
        let id = commands
            .spawn((
                Transform {
                    translation: pivots[i] - parent_pos,
                    rotation: rotations[i],
                    scale: Vec3::ONE,
                },
                Name::new((*bone).to_string()),
            ))
            .id();
        let parent = PARENT[i].map_or(root, |p| joints[p]);
        commands.entity(parent).add_child(id);
        joints.push(id);
    }

    // The eyes lie at the nose bridge, a little in front of the head's centre line (forward is
    // -Z in the character's frame).
    let eye_rel = Vec3::new(
        0.0,
        (profile.height_m - EYE_BELOW_CROWN_M) - profile.links.neck.y,
        -EYES_FORWARD_M,
    );
    let head_anchor = commands
        .spawn((
            Transform::from_translation(eye_rel),
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
                (
                    Mesh3d(part.mesh.clone()),
                    MeshMaterial3d(part.material.clone()),
                ),
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
mod tests;
