//! Animation pivots: the pivot of each mesh, the frames that reference one another and the bone stems.

use super::*;

/// One entity per `[mesh_ident]` that some `[animparent]` of the segment names: the frame the
/// parent mesh animates and its children live in (names compare case-insensitively). Spawned
/// below `root` with an identity transform; the loop reparents and animates them in file order,
/// so a child may come before its parent.
pub(in super::super) fn anim_pivots(
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
pub(in super::super) fn referenced_idents(
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
pub(in super::super) fn forms_cycle(
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

pub(in super::super) fn pivot_of(
    pivots: &std::collections::HashMap<String, Entity>,
    ident: Option<&str>,
) -> Option<Entity> {
    pivots.get(&ident?.trim().to_lowercase()).copied()
}

/// The mesh's own origin in OMSI vehicle axes, for animations with `origin_from_mesh`: the
/// translation row of the `.o3d` matrix chunk (o3d axes x lateral, y up, z longitudinal).
pub(in super::super) fn mesh_pivot(
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
        scale: if scale.is_finite() && scale > 1e-6 {
            scale
        } else {
            1.0
        },
    })
}

/// Whether a mesh file stem names a bone dummy of a bellows (`bone_a` … `bone_f`).
pub(in super::super) fn is_bone_stem(stem: &str) -> bool {
    // `bone_a` … `bone_d` (Heuliez), `dummya` … `dummyd` (MAN New Lion's City), `dummy_a`.
    ["bone_", "dummy_", "dummy"].iter().any(|prefix| {
        stem.strip_prefix(prefix)
            .is_some_and(|rest| rest.len() == 1 && rest.chars().all(|c| c.is_ascii_alphabetic()))
    })
}
