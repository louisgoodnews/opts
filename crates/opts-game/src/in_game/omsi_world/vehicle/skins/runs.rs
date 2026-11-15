//! Articulation runs: the joints of a frame and the runs of dummies that number a skinned mesh's bones.

use super::*;

/// The run of numbered dummies a skinned mesh numbers its bones along: the run touched last if it
/// is long enough for the highest `[setbone]` number, else the most recent one that is (damper
/// pairs `…_l`/`…_r` form short runs), else the run touched last.
pub(in super::super) fn pick_run(
    runs: &[(String, Vec<Entity>)],
    last_run: Option<usize>,
    bones: &[opts_core::formats::vehicles::model::MeshBone],
) -> Vec<Entity> {
    let need = bones
        .iter()
        .filter_map(|b| usize::try_from(b.index).ok())
        .max()
        .map_or(0, |m| m + 1);
    let mut order: Vec<usize> = (0..runs.len()).rev().collect();
    if let Some(at) = last_run {
        order.retain(|i| *i != at);
        order.insert(0, at);
    }
    order
        .into_iter()
        .map(|i| &runs[i].1)
        .find(|r| r.len() >= need)
        .or_else(|| last_run.and_then(|at| runs.get(at)).map(|r| &r.1))
        .cloned()
        .unwrap_or_default()
}

/// The joint list of a skin whose parts hang in `frame`. A skin in the vehicle frame uses the
/// dummies themselves. A skin below an `[animparent]` pivot (the rear half of the Valiant and
/// Heuliez bellows, `[animparent] tourelle`) moves its bones in that frame, as its parts do: each
/// dummy is repeated as a child of the frame with the same animation (`proxy`), and the bones
/// that stay on the vehicle stay on the frame. Bevy ignores the transform of a skinned mesh, so
/// without this the half followed the dummies alone and missed the trailer by half the angle.
pub(in super::super) fn joints_in_frame(
    joints: &[Entity],
    root: Entity,
    frame: Entity,
    mut proxy: impl FnMut(Entity) -> Entity,
) -> Vec<Entity> {
    if frame == root {
        return joints.to_vec();
    }
    let mut made: Vec<(Entity, Entity)> = Vec::new();
    joints
        .iter()
        .map(|&j| {
            if j == root {
                return frame;
            }
            if let Some(&(_, p)) = made.iter().find(|(d, _)| *d == j) {
                return p;
            }
            let p = proxy(j);
            made.push((j, p));
            p
        })
        .collect()
}

/// Marks the meshes that belong to a run of numbered dummies: two or more meshes of the model whose
/// file stems differ only in their last letter or digit and whose animations are driven by
/// `articulation_*` variables (the bones of a bellows in the order of their `[setbone]` index).
pub(in super::super) fn articulation_runs(
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
