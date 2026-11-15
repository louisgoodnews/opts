//! Reading the runtime state of the object: script variables, mesh animations, lights and light maps.

use super::*;
use super::types::Handles;

/// Reads the runtime's state of the object at `root`.
pub(super) fn probe(world: &World, root: Entity, sco: &SceneryObject, handles: &Handles) -> Probe {
    let mut p = Probe {
        shown: vec![Vec::new(); sco.meshes.len()],
        ..Probe::default()
    };
    if let Some(scripts) = world.get::<SceneryScripts>(root) {
        p.vars = scripts
            .with_engine(|e| e.variables())
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| (k.to_lowercase(), v))
            .collect();
    }
    for &e in &handles.meshes {
        let index = world
            .get::<Name>(e)
            .and_then(|n| n.as_str().strip_prefix("Mesh_"))
            .and_then(|s| s.parse::<usize>().ok());
        if let (Some(i), Some(rule)) = (index, world.get::<SceneryMeshRule>(e)) {
            if let Some(slot) = p.shown.get_mut(i) {
                slot.clone_from(&rule.anim.shown);
            }
        }
    }
    for &e in &handles.lights {
        if let (Some(l), Some(g), Some(v)) = (
            world.get::<SceneryLight>(e),
            world.get::<GlobalTransform>(e),
            world.get::<Visibility>(e),
        ) {
            p.lights.push(LightProbe {
                level: l.level,
                shown: *v != Visibility::Hidden,
                pos: g.translation(),
                size_m: l.size_m,
            });
        }
    }
    for &e in &handles.light_maps {
        if let Some(m) = world.get::<SceneryLightMap>(e) {
            p.light_maps.push(m.applied);
        }
    }
    p
}
