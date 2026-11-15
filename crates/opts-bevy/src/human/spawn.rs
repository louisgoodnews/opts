//! Spawning a human: the root, the joint hierarchy, the head anchor and the skinned mesh parts.

use super::*;

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
