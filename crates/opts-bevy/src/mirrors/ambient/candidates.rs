use bevy::prelude::*;

use crate::mirrors::*;

use super::{AMBIENT_MIN_FACING, AMBIENT_RANGE_M};

/// A pane that is a candidate this frame.
pub(super) struct Candidate {
    pub(super) pane: Entity,
    pub(super) distance: f32,
    pub(super) centre: Vec3,
    pub(super) normal: Vec3,
    pub(super) height: f32,
    pub(super) aspect: f32,
}

pub(super) fn collect_candidates(
    panes: &Query<
        (
            Entity,
            &AmbientPane,
            &GlobalTransform,
            Option<&InheritedVisibility>,
        ),
        Without<MirrorSurface>,
    >,
    viewer: Vec3,
    forward: Vec3,
    parents: &Query<&ChildOf>,
    players: &Query<(), With<PlayerVehicle>>,
    budget: usize,
) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = Vec::new();
    for (entity, pane, gt, visible) in panes {
        if visible.is_some_and(|v| !v.get()) {
            continue;
        }
        let centre = gt.transform_point(pane.centre);
        let distance = centre.distance(viewer);
        if distance > AMBIENT_RANGE_M || !mirror_in_front(viewer, forward, centre) {
            continue;
        }
        let normal = gt
            .affine()
            .matrix3
            .mul_vec3(pane.normal)
            .normalize_or_zero();
        let to_viewer = (viewer - centre).normalize_or_zero();
        if normal.dot(to_viewer).abs() < AMBIENT_MIN_FACING {
            continue;
        }
        if parents.iter_ancestors(entity).any(|a| players.contains(a)) {
            continue;
        }
        let (scale, _, _) = gt.to_scale_rotation_translation();
        candidates.push(Candidate {
            pane: entity,
            distance,
            centre,
            normal,
            height: pane.height * scale.max_element(),
            aspect: pane.aspect,
        });
    }
    candidates.sort_by(|a, b| a.distance.total_cmp(&b.distance).then(a.pane.cmp(&b.pane)));
    candidates.truncate(budget);
    candidates
}
