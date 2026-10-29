use bevy::prelude::*;

use crate::mirrors::schedule::{mirror_in_view, MIRROR_MIN_ANGLE};
use crate::mirrors::*;

/// A group's pose in the world for the current frame.
pub(super) struct GroupPose {
    pub(super) camera: Entity,
    pub(super) centre: Vec3,
    pub(super) normal: Vec3,
    pub(super) height: f32,
    pub(super) aspect: f32,
    pub(super) distance: f32,
}

pub(super) fn compute_group_poses(
    live: &[Entity],
    cameras: &Query<(
        Entity,
        &mut MirrorCamera,
        &mut Transform,
        &mut Camera,
        &mut Projection,
    )>,
    panes: &Query<(&GlobalTransform, Option<&InheritedVisibility>), With<MirrorSurface>>,
    viewer: Vec3,
) -> Vec<GroupPose> {
    let mut poses = Vec::with_capacity(live.len());
    for e in live {
        let Ok((_, info, ..)) = cameras.get(*e) else {
            continue;
        };
        let (mut centre, mut normal, mut height, mut aspect, mut n) =
            (Vec3::ZERO, Vec3::ZERO, 0.0f32, 0.0f32, 0u32);
        for m in &info.members {
            let Ok((gt, visible)) = panes.get(m.entity) else {
                continue;
            };
            if visible.is_some_and(|v| !v.get()) {
                continue;
            }
            let (scale, _, _) = gt.to_scale_rotation_translation();
            centre += gt.transform_point(m.centre);
            normal += gt.affine().matrix3.mul_vec3(m.normal).normalize_or_zero();
            height = height.max(m.height * scale.max_element());
            aspect += m.aspect;
            n += 1;
        }
        let distance = if n == 0 {
            f32::INFINITY
        } else {
            let c = centre / n as f32;
            c.distance(viewer)
        };
        poses.push(GroupPose {
            camera: *e,
            centre: if n == 0 {
                Vec3::ZERO
            } else {
                centre / n as f32
            },
            normal: normal.normalize_or_zero(),
            height,
            aspect: if n == 0 { 1.0 } else { aspect / n as f32 },
            distance,
        });
    }
    poses
}

pub(super) fn compute_in_view(
    poses: &[GroupPose],
    count: u8,
    view: &GlobalTransform,
    main_projection: Option<&Projection>,
) -> Vec<bool> {
    let (viewer, forward) = (view.translation(), view.forward().as_vec3());
    poses
        .iter()
        .map(|p| {
            if count == 0 || !p.distance.is_finite() {
                return false;
            }
            let radius = 0.5 * p.height * (1.0 + p.aspect * p.aspect).sqrt();
            if 2.0 * (p.height / (2.0 * p.distance.max(0.01))).atan() < MIRROR_MIN_ANGLE {
                return false;
            }
            match main_projection {
                Some(Projection::Perspective(persp)) => {
                    mirror_in_view(view, persp.fov, persp.aspect_ratio, p.centre, radius)
                }
                _ => mirror_in_front(viewer, forward, p.centre),
            }
        })
        .collect()
}
