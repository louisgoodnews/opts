//! Animation pivots, articulation runs and skin binding.

use super::*;

/// One entity per `[mesh_ident]` that some `[animparent]` of the segment names: the frame the
/// parent mesh animates and its children live in (names compare case-insensitively). Spawned
/// below `root` with an identity transform; the loop reparents and animates them in file order,
/// so a child may come before its parent.
pub(super) fn anim_pivots(
    commands: &mut Commands,
    root: Entity,
    meshes: &[opts_core::formats::vehicles::model::ModelMesh],
) -> std::collections::HashMap<String, Entity> {
    let wanted = referenced_idents(meshes);
    let mut pivots = std::collections::HashMap::new();
    for ident in &wanted {
        let pivot = commands
            .spawn((
                Name::new(format!("anim_{ident}")),
                Transform::IDENTITY,
                Visibility::Inherited,
            ))
            .id();
        commands.entity(root).add_child(pivot);
        pivots.insert(ident.clone(), pivot);
    }
    pivots
}

/// The lower-case `[mesh_ident]`s that are declared by a mesh and named by another mesh's
/// `[animparent]`.
pub(super) fn referenced_idents(
    meshes: &[opts_core::formats::vehicles::model::ModelMesh],
) -> std::collections::BTreeSet<String> {
    let declared: std::collections::BTreeSet<String> = meshes
        .iter()
        .filter_map(|m| m.mesh_ident.as_deref())
        .map(|i| i.trim().to_lowercase())
        .collect();
    meshes
        .iter()
        .filter_map(|m| m.animparent.as_deref())
        .map(|i| i.trim().to_lowercase())
        .filter(|i| declared.contains(i))
        .collect()
}

/// Whether hanging a mesh named `ident` below `parent` closes a loop of `[animparent]`s (the
/// mesh itself, or a chain of parents that leads back to it).
pub(super) fn forms_cycle(
    meshes: &[opts_core::formats::vehicles::model::ModelMesh],
    ident: Option<&str>,
    parent: Option<&str>,
) -> bool {
    let (Some(ident), Some(mut at)) = (ident, parent) else {
        return false;
    };
    let same = |a: &str, b: &str| a.trim().eq_ignore_ascii_case(b.trim());
    for _ in 0..=meshes.len() {
        if same(at, ident) {
            return true;
        }
        let up = meshes
            .iter()
            .find(|m| m.mesh_ident.as_deref().is_some_and(|i| same(i, at)))
            .and_then(|m| m.animparent.as_deref());
        match up {
            Some(next) => at = next,
            None => return false,
        }
    }
    true
}

pub(super) fn pivot_of(
    pivots: &std::collections::HashMap<String, Entity>,
    ident: Option<&str>,
) -> Option<Entity> {
    pivots.get(&ident?.trim().to_lowercase()).copied()
}

/// A skinned mesh waiting for its bones: its entities, its `[setbone]` list and the run of
/// `bone_<letter>` dummies declared before it.
pub(super) struct SkinBinding {
    pub(super) entities: Vec<Entity>,
    pub(super) bones: Vec<opts_core::formats::vehicles::model::MeshBone>,
    pub(super) run: Vec<Entity>,
}

/// Marks the meshes that belong to a run of numbered dummies: two or more meshes of the model whose
/// file stems differ only in their last letter or digit and whose animations are driven by
/// `articulation_*` variables (the bones of a bellows in the order of their `[setbone]` index).
pub(super) fn articulation_runs(
    meshes: &[opts_core::formats::vehicles::model::ModelMesh],
) -> Vec<bool> {
    let key = |m: &opts_core::formats::vehicles::model::ModelMesh| -> Option<String> {
        let driven = m.anims.iter().any(|a| {
            a.driver_variable()
                .is_some_and(|v| v.to_ascii_lowercase().starts_with("articulation_"))
        });
        let stem = Path::new(&m.path.replace('\\', "/"))
            .file_stem()?
            .to_string_lossy()
            .to_lowercase();
        let mut chars = stem.chars();
        let last = chars.next_back()?;
        (driven && last.is_ascii_alphanumeric() && stem.chars().count() >= 3)
            .then(|| chars.as_str().to_owned())
    };
    let keys: Vec<Option<String>> = meshes.iter().map(key).collect();
    let mut count: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for k in keys.iter().flatten() {
        *count.entry(k.as_str()).or_default() += 1;
    }
    keys.iter()
        .map(|k| k.as_deref().is_some_and(|k| count[k] >= 2))
        .collect()
}

/// Whether a mesh file stem names a bone dummy of a bellows (`bone_a` … `bone_f`).
pub(super) fn is_bone_stem(stem: &str) -> bool {
    // `bone_a` … `bone_d` (Heuliez), `dummya` … `dummyd` (MAN New Lion's City), `dummy_a`.
    ["bone_", "dummy_", "dummy"].iter().any(|prefix| {
        stem.strip_prefix(prefix)
            .is_some_and(|rest| rest.len() == 1 && rest.chars().all(|c| c.is_ascii_alphabetic()))
    })
}

/// The mesh's own origin in OMSI vehicle axes, for animations with `origin_from_mesh`: the
/// translation row of the `.o3d` matrix chunk (o3d axes x lateral, y up, z longitudinal).
pub(super) fn mesh_pivot(
    job: &WorldLoadJob,
    rel: &str,
    model_dir: &Path,
    mesh_def: &opts_core::formats::vehicles::model::ModelMesh,
) -> Option<opts_bevy::MeshPivot> {
    if !mesh_def.anims.iter().any(|a| a.origin_from_mesh) {
        return None;
    }
    let path = job.install.find_in(model_dir, rel)?;
    let o3d = opts_core::formats::o3d::parse(&std::fs::read(path).ok()?).ok()?;
    let m = o3d.transform?;
    // Rotation part of the matrix: normalised rows (images of the local axes, o3d coordinates).
    let row = |i: usize| {
        let v = Vec3::new(m[i][0], m[i][1], m[i][2]);
        let v = v.try_normalize().unwrap_or(Vec3::ZERO);
        [v.x, v.y, v.z]
    };
    let scale = (0..3)
        .map(|i| Vec3::new(m[i][0], m[i][1], m[i][2]).length())
        .sum::<f32>()
        / 3.0;
    Some(opts_bevy::MeshPivot {
        origin: Vec3::new(m[3][0], m[3][2], m[3][1]),
        rows: [row(0), row(1), row(2)],
        scale: if scale.is_finite() && scale > 1e-6 { scale } else { 1.0 },
    })
}

/// Binds skinned meshes of a vehicle to the dummy meshes that carry their bones' animation.
///
/// OMSI names a bone in `[setbone] <name> <index>`; the dummy mesh of that bone is the one whose
/// file name contains the bone name (`GN_Dum_Gelenk_A.o3d` for `Gelenk_A`). The bone matrix is
/// the dummy's animation matrix (the vertices are in vehicle space), so the inverse bind poses
/// are the identity. A bone without a dummy stays on the vehicle root (not animated).
pub(super) fn bind_skins(
    commands: &mut Commands,
    root: Entity,
    dummies: &[(String, Entity)],
    skins: Vec<SkinBinding>,
) {
    for SkinBinding {
        entities,
        bones,
        run,
    } in skins
    {
        let run_stems: Vec<(&String, Entity)> = dummies
            .iter()
            .filter(|(_, e)| run.contains(e))
            .map(|(stem, e)| (stem, *e))
            .collect();
        let n = bones
            .iter()
            .map(|b| usize::try_from(b.index).map_or(0, |i| i + 1))
            .max()
            .unwrap_or(0);
        if n == 0 {
            continue;
        }
        // One extra joint after the highest: bones of the file the model does not list (the
        // base bone of a bellows) stay on the vehicle.
        let mut joints = vec![root; n + 1];
        for b in &bones {
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
        commands.queue(move |world: &mut World| {
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
