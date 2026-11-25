//! Counting the scene: the cameras with their views and the mesh parts of the main world.

use super::*;

/// Counts of the mesh parts in the main world.
/// Every camera with its state: each active one is another view the whole scene is culled and
/// shadowed for (M-0029: the driver's seat costs eight shadow views, not four).
pub(super) fn cameras(world: &mut World) -> Vec<Value> {
    let mut q = world.query::<(Option<&Name>, &Camera, Option<&RenderTarget>)>();
    q.iter(world)
        .map(|(name, camera, target)| {
            json!({
                "name": name.map(|n| n.as_str().to_owned()),
                "active": camera.is_active,
                "order": camera.order,
                "target": match target {
                    Some(RenderTarget::Image(_)) => "image",
                    Some(_) => "other",
                    None => "default",
                },
            })
        })
        .collect()
}

pub(super) fn parts(world: &mut World) -> Value {
    let materials = world.resource::<Assets<StandardMaterial>>();
    let alpha: HashMap<AssetId<StandardMaterial>, u8> = materials
        .iter()
        .map(|(id, m)| {
            let kind = match m.alpha_mode {
                AlphaMode::Opaque => 0,
                AlphaMode::Mask(_) => 1,
                _ => 2,
            };
            (id, kind)
        })
        .collect();
    let mut q = world.query::<(
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        Option<&ViewVisibility>,
        Has<NoAutomaticBatching>,
    )>();
    let (mut total, mut visible, mut no_batch) = (0usize, 0usize, 0usize);
    let mut meshes = HashSet::new();
    let mut mats = HashSet::new();
    let mut pairs = HashSet::new();
    let mut per_kind = [0usize; 3];
    let mut mat_users: HashMap<AssetId<StandardMaterial>, usize> = HashMap::new();
    for (mesh, mat, vis, nb) in q.iter(world) {
        total += 1;
        if !vis.is_none_or(|v| v.get()) {
            continue;
        }
        visible += 1;
        no_batch += usize::from(nb);
        meshes.insert(mesh.0.id());
        mats.insert(mat.0.id());
        pairs.insert((mesh.0.id(), mat.0.id()));
        per_kind[usize::from(*alpha.get(&mat.0.id()).unwrap_or(&0))] += 1;
        *mat_users.entry(mat.0.id()).or_default() += 1;
    }
    let single_use = mat_users.values().filter(|n| **n == 1).count();
    let cameras = cameras(world);
    let mut casters = world.query_filtered::<(), (With<Mesh3d>, Without<NotShadowCaster>)>();
    let shadow_casters = casters.iter(world).count();
    json!({
        "mesh_entities": total,
        "visible": visible,
        "visible_distinct_meshes": meshes.len(),
        "visible_distinct_materials": mats.len(),
        "visible_distinct_pairs": pairs.len(),
        "visible_materials_used_once": single_use,
        "visible_opaque_mask_blend": per_kind,
        "no_automatic_batching": no_batch,
        "shadow_casters": shadow_casters,
        "cameras": cameras,
    })
}
