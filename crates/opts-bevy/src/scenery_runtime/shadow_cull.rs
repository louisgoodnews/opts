//! Per-part shadow culling of scenery objects (M-0029, `docs/src/architecture/lod-culling.md`
//! §3.1): a mesh part of a placed scenery object stops casting the sun's shadow once it is
//! farther from the main camera than [`ShadowDetail::factor`] times its own radius, so small
//! parts drop out of the shadow cascades long before large ones.
//!
//! Every mesh added below a [`SceneryDrawRoot`] becomes a [`ShadowCullCandidate`], except one
//! that already has `NotShadowCaster` (effects). The cull pass adds and removes
//! `NotShadowCaster` together with its own [`ShadowCulled`] marker, so it never re-enables a
//! part another system made a non-caster.

use bevy::camera::primitives::Aabb;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use super::SceneryDrawRoot;

/// Fraction of the cast distance a culled part must come back inside before it casts again.
pub const SHADOW_HYSTERESIS: f32 = 0.1;
/// Seconds between two culling passes.
const INTERVAL_S: f32 = 0.25;

/// How far small scenery parts cast shadows, from `graphics.shadow_detail`.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct ShadowDetail {
    /// Metres of cast distance per metre of part radius; `None` lets every part cast.
    pub factor: Option<f32>,
}

/// A mesh part of a placed scenery object that the shadow culling may switch.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ShadowCullCandidate;

/// Marks a part made a non-caster by the shadow culling (and only those).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ShadowCulled;

/// Whether a part of `radius` at `distance` stops casting under `factor`; `was_culled` applies
/// the hysteresis so a part at the border does not toggle.
#[must_use]
pub fn is_shadow_culled(distance: f32, radius: f32, factor: f32, was_culled: bool) -> bool {
    let reach = factor * radius;
    if was_culled {
        distance > reach * (1.0 - SHADOW_HYSTERESIS)
    } else {
        distance > reach
    }
}

/// Marks meshes added below a scenery object root as shadow-culling candidates.
pub fn mark_shadow_candidates(
    added: Query<Entity, (Added<Mesh3d>, Without<NotShadowCaster>)>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<SceneryDrawRoot>>,
    mut commands: Commands,
) {
    for entity in &added {
        if parents.iter_ancestors(entity).any(|a| roots.contains(a)) {
            commands.entity(entity).insert(ShadowCullCandidate);
        }
    }
}

type ShadowPart<'a> = (Entity, &'a GlobalTransform, &'a Aabb, Has<ShadowCulled>);

/// Switches candidates' shadows by distance and size every `INTERVAL_S`, and at once when the
/// setting changes; with the setting off every part it culled casts again.
pub fn scenery_shadow_cull_system(
    time: Res<Time>,
    detail: Res<ShadowDetail>,
    mut since: Local<f32>,
    cameras: Query<(&Camera, &GlobalTransform), With<crate::MainCamera>>,
    parts: Query<ShadowPart<'_>, With<ShadowCullCandidate>>,
    mut commands: Commands,
) {
    *since += time.delta_secs();
    if *since < INTERVAL_S && !detail.is_changed() {
        return;
    }
    *since = 0.0;
    let camera = cameras
        .iter()
        .find(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation());
    for (entity, transform, aabb, culled) in &parts {
        let want = match (detail.factor, camera) {
            (Some(factor), Some(cam)) => {
                let (scale, _, _) = transform.to_scale_rotation_translation();
                let radius = Vec3::from(aabb.half_extents).length() * scale.abs().max_element();
                let centre = transform.transform_point(Vec3::from(aabb.center));
                is_shadow_culled(centre.distance(cam), radius, factor, culled)
            }
            (Some(_), None) => culled,
            (None, _) => false,
        };
        if want == culled {
            continue;
        }
        if want {
            commands
                .entity(entity)
                .insert((NotShadowCaster, ShadowCulled));
        } else {
            commands
                .entity(entity)
                .remove::<(NotShadowCaster, ShadowCulled)>();
        }
    }
}

#[cfg(test)]
mod tests;
