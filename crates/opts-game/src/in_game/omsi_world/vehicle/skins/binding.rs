//! Binding skinned meshes to their bones: the skin binding and the joints it is assigned.

use super::*;

/// A skinned mesh waiting for its bones: its entities, its `[setbone]` list, the run of
/// `bone_<letter>` dummies declared before it and the frame its parts hang in (the vehicle root,
/// or the pivot of its `[animparent]`).
pub(in super::super) struct SkinBinding {
    pub(in super::super) entities: Vec<Entity>,
    pub(in super::super) bones: Vec<opts_core::formats::vehicles::model::MeshBone>,
    pub(in super::super) run: Vec<Entity>,
    pub(in super::super) frame: Entity,
}

/// The entity each joint of a skin follows: the dummy named like the bone, else the dummy of the
/// same trailing letter in the run, else the dummy at the bone's number in the run; unmatched
/// bones stay on `root`.
fn assign_joints(
    root: Entity,
    dummies: &[(String, Entity)],
    (bones, run): (&[opts_core::formats::vehicles::model::MeshBone], &[Entity]),
    n: usize,
) -> Vec<Entity> {
    let run_stems: Vec<(&String, Entity)> = dummies
        .iter()
        .filter(|(_, e)| run.contains(e))
        .map(|(stem, e)| (stem, *e))
        .collect();
    // One extra joint after the highest: bones of the file the model does not list (the
    // base bone of a bellows) stay on the vehicle.
    let mut joints = vec![root; n + 1];
    for b in bones {
        let Ok(i) = usize::try_from(b.index) else {
            continue;
        };
        let name = b.name.to_lowercase();
        // By name (`Gelenk_A.o3d` for `Gelenk_A`), else the dummy of the same trailing letter
        // or digit (`Bone_C` ↔ `Dum_1_C`), else the dummy at the bone's number in the run.
        let suffix = name
            .rsplit(['_', ' '])
            .next()
            .filter(|t| t.chars().count() == 1);
        let lettered = suffix.and_then(|t| {
            run_stems
                .iter()
                .find(|(stem, _)| stem.ends_with(t))
                .map(|(_, e)| *e)
        });
        match dummies.iter().find(|(stem, _)| stem.contains(&name)) {
            Some((_, e)) => joints[i] = *e,
            None => match lettered.or_else(|| run.get(i).copied()) {
                Some(e) => joints[i] = e,
                None => {
                    tracing::warn!(bone = %b.name, index = b.index, run = run.len(), dummies = ?dummies.iter().map(|d| d.0.as_str()).collect::<Vec<_>>(), "no dummy mesh for bone; it stays on the root");
                }
            },
        }
    }
    joints
}

/// Binds skinned meshes of a vehicle to the dummy meshes that carry their bones' animation.
///
/// OMSI names a bone in `[setbone] <name> <index>`; the dummy mesh of that bone is the one whose
/// file name contains the bone name (`GN_Dum_Gelenk_A.o3d` for `Gelenk_A`). The bone matrix is
/// the dummy's animation matrix (the vertices are in vehicle space), so the inverse bind poses
/// are the identity. A bone without a dummy stays on the vehicle root (not animated).
pub(in super::super) fn bind_skins(
    commands: &mut Commands,
    root: Entity,
    dummies: &[(String, Entity)],
    skins: Vec<SkinBinding>,
) {
    for SkinBinding {
        entities,
        bones,
        run,
        frame,
    } in skins
    {
        let n = bones
            .iter()
            .map(|b| usize::try_from(b.index).map_or(0, |i| i + 1))
            .max()
            .unwrap_or(0);
        if n == 0 {
            continue;
        }
        let joints = assign_joints(root, dummies, (&bones, &run), n);
        commands.queue(move |world: &mut World| {
            let joints = joints_in_frame(&joints, root, frame, |dummy| {
                let anim = world.get::<opts_bevy::MeshAnimation>(dummy).cloned();
                let name = world
                    .get::<Name>(dummy)
                    .map_or_else(|| "joint".to_owned(), |n| n.as_str().to_owned());
                let mut proxy = world.spawn((
                    Name::new(format!("{name}_in_frame")),
                    Transform::IDENTITY,
                    Visibility::Inherited,
                    ChildOf(frame),
                ));
                if let Some(anim) = anim {
                    proxy.insert(anim);
                }
                proxy.id()
            });
            let handle = world
                .resource_mut::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>()
                .add(bevy::mesh::skinning::SkinnedMeshInverseBindposes::from(
                    vec![Mat4::IDENTITY; joints.len()],
                ));
            for e in entities {
                if let Ok(mut entity) = world.get_entity_mut(e) {
                    entity.insert((
                        bevy::mesh::skinning::SkinnedMesh {
                            inverse_bindposes: handle.clone(),
                            joints: joints.clone(),
                        },
                        bevy::camera::visibility::NoFrustumCulling,
                    ));
                }
            }
        });
    }
}
