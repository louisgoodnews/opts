//! The ambient camera pool's per-frame work: handing free cameras a pane, and posing the
//! cameras of the chosen panes (switching each on or off for this frame).

use bevy::prelude::*;

use super::candidates::Candidate;
use super::panes::AmbientPaneItem;
use super::spawn::mirror_material;
use crate::mirrors::*;

/// A camera of the pool, and the pane it serves.
#[derive(Component, Debug, Clone)]
pub struct AmbientCamera {
    /// The picture it renders into.
    pub picture: Handle<Image>,
    /// The material that shows the picture on the pane.
    pub dynamic: Handle<StandardMaterial>,
    /// The pane it serves now.
    pub pane: Option<Entity>,
}

pub(super) type AmbientCameraItem<'a> = (
    Entity,
    &'a mut AmbientCamera,
    &'a mut Transform,
    &'a mut Camera,
    &'a mut Projection,
);

pub(super) type MainCameraAmbientFilter = (
    With<MainCamera>,
    Without<AmbientCamera>,
    Without<MirrorCamera>,
);

pub(super) fn assign_free_cameras(
    free: Vec<Entity>,
    waiting: impl Iterator<Item = Entity>,
    cameras: &mut Query<AmbientCameraItem>,
    panes: &Query<AmbientPaneItem, Without<MirrorSurface>>,
    materials: &mut Assets<StandardMaterial>,
    settings: &MirrorSettings,
) {
    let mut waiting = waiting;
    for camera in free {
        let Some(pane_entity) = waiting.next() else {
            break;
        };
        let Ok((_, mut info, ..)) = cameras.get_mut(camera) else {
            continue;
        };
        if let Ok((_, pane, ..)) = panes.get(pane_entity) {
            if let Some(base) = materials.get(&pane.original).cloned() {
                let new = mirror_material(
                    &base,
                    info.picture.clone(),
                    pane.uv_min,
                    pane.uv_max,
                    settings,
                );
                if let Some(mut m) = materials.get_mut(&info.dynamic) {
                    *m = new;
                }
            }
        }
        info.pane = Some(pane_entity);
    }
}

pub(super) fn pose_ambient_cameras(
    cameras: &mut Query<AmbientCameraItem>,
    candidates: &[Candidate],
    viewer: Vec3,
    frame: u64,
    every: u8,
    pane_materials: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
    stats: &mut MirrorStats,
) {
    let mut rank = 0;
    for (_, info, mut t, mut cam, mut projection) in cameras.iter_mut() {
        let Some(pane) = info.pane else {
            if cam.is_active {
                cam.is_active = false;
            }
            continue;
        };
        let Some(c) = candidates.iter().find(|c| c.pane == pane) else {
            continue;
        };
        if let Ok(mut m) = pane_materials.get_mut(pane) {
            if m.0 != info.dynamic {
                m.0 = info.dynamic.clone();
            }
        }
        if let Some((position, direction)) = mirror_camera_pose(viewer, c.centre, c.normal) {
            t.translation = position;
            t.look_to(direction, Vec3::Y);
        }
        if let Projection::Perspective(persp) = &mut *projection {
            persp.fov = mirror_fov(c.height, c.distance);
            persp.aspect_ratio = c.aspect.clamp(MIRROR_ASPECT_RANGE.0, MIRROR_ASPECT_RANGE.1);
        }
        let render = is_render_frame(frame, rank, every);
        if cam.is_active != render {
            cam.is_active = render;
        }
        stats.ambient += 1;
        rank += 1;
    }
}
