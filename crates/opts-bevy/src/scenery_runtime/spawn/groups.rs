//! The LOD groups of an object and the bounds of its parts.

use super::*;

/// Spawns the LOD group entities (one per distinct mesh threshold) under `root`. Light sprites
/// are not part of a LOD group: a lantern's `[light_enh_2]` written after `[LOD] 0` would
/// otherwise vanish with its mesh band within ~70 m, while the glow is what is seen from afar.
pub(super) fn lod_groups(
    commands: &mut Commands,
    root: Entity,
    sco: &SceneryObject,
) -> Vec<(Entity, f32)> {
    let mut thresholds: Vec<f32> = Vec::new();
    let all = sco.meshes.iter().map(|m| m.lod_threshold);
    for t in all.flatten() {
        if !thresholds.contains(&t) {
            thresholds.push(t);
        }
    }
    thresholds
        .into_iter()
        .map(|t| {
            let e = commands
                .spawn((
                    Transform::default(),
                    Visibility::Inherited,
                    Name::new(format!("LOD_{t}")),
                ))
                .id();
            commands.entity(root).add_child(e);
            (e, t)
        })
        .collect()
}

/// Bounding sphere (centre, radius) of all parts in the object's frame.
pub(super) fn bounds(parts: &[Vec<ModelPart>], meshes: &Assets<Mesh>) -> (Vec3, f32) {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for part in parts.iter().flatten() {
        let positions = meshes
            .get(&part.mesh)
            .and_then(|m| m.attribute(Mesh::ATTRIBUTE_POSITION));
        if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) = positions {
            for p in pos {
                lo = lo.min(Vec3::from(*p));
                hi = hi.max(Vec3::from(*p));
            }
        }
    }
    if lo.x > hi.x {
        return (Vec3::ZERO, 1.0);
    }
    ((lo + hi) * 0.5, ((hi - lo) * 0.5).length().max(0.1))
}
