use bevy::prelude::*;

use crate::mirrors::*;

use super::pose::GroupPose;

pub(super) fn update_virtual_slots(
    poses: &[GroupPose],
    cameras: &Query<(
        Entity,
        &mut MirrorCamera,
        &mut Transform,
        &mut Camera,
        &mut Projection,
    )>,
    vehicles: &Query<&GlobalTransform, With<PlayerVehicle>>,
    virtual_mirrors: &mut VirtualMirrors,
) -> Vec<Entity> {
    let candidates: Vec<(Entity, Vec3)> = poses
        .iter()
        .filter_map(|p| {
            let info = cameras.get(p.camera).ok()?.1;
            if let Some(tag) = &info.tag {
                return Some((p.camera, Vec3::new(tag.x, tag.z, -tag.y)));
            }
            let vehicle = vehicles.get(info.vehicle).ok()?;
            p.distance.is_finite().then(|| {
                (
                    p.camera,
                    vehicle.affine().inverse().transform_point3(p.centre),
                )
            })
        })
        .collect();
    let slots = assign_slots(&candidates);
    let mut streaming: Vec<Entity> = Vec::new();
    for slot in MirrorSlot::ALL {
        let i = slot.index();
        virtual_mirrors.feeds[i] = slots[i].and_then(|camera| {
            let (_, info, ..) = cameras.get(camera).ok()?;
            let aspect = info.members.iter().map(|m| m.aspect).sum::<f32>()
                / info.members.len().max(1) as f32;
            Some(MirrorFeed {
                camera,
                picture: info.picture.clone(),
                aspect: if info.tag.is_some() { 1.0 } else { aspect },
            })
        });
        if let Some(camera) = virtual_mirrors.streaming_camera(slot) {
            streaming.push(camera);
        }
    }
    streaming
}

fn apply_single_pose(
    p: &GroupPose,
    info: &MirrorCamera,
    t: &mut Transform,
    projection: &mut Projection,
    settings: &MirrorSettings,
    viewer: Vec3,
    vehicles: &Query<&GlobalTransform, With<PlayerVehicle>>,
) {
    if let (Some(tag), Ok(vehicle)) = (&info.tag, vehicles.get(info.vehicle)) {
        let (local, rotation, fov) = tag_pose(tag, &settings.adjust);
        let (_, vrot, vpos) = vehicle.to_scale_rotation_translation();
        t.translation = vpos + vrot * local;
        t.rotation = vrot * rotation;
        if let Projection::Perspective(persp) = &mut *projection {
            if (persp.fov - fov).abs() > 1e-4 {
                persp.fov = fov;
            }
            persp.aspect_ratio = 1.0;
        }
        return;
    }
    if let Some((position, direction)) = mirror_camera_pose(viewer, p.centre, p.normal) {
        t.translation = position;
        t.look_to(direction, Vec3::Y);
    }
    if let Projection::Perspective(persp) = &mut *projection {
        let fov = mirror_fov(p.height, p.distance);
        if (persp.fov - fov).abs() > 1e-3 {
            persp.fov = fov;
        }
        let aspect = p.aspect.clamp(MIRROR_ASPECT_RANGE.0, MIRROR_ASPECT_RANGE.1);
        if (persp.aspect_ratio - aspect).abs() > 1e-3 {
            persp.aspect_ratio = aspect;
        }
    }
}

pub(super) fn apply_camera_renders_and_poses(
    params: &mut super::MirrorUpdateParams,
    poses: &[GroupPose],
    picked: &[usize],
    streaming: &[Entity],
    count: u8,
    viewer: Vec3,
) {
    let frame = params.frame.0;
    let super::MirrorUpdateParams {
        cameras,
        images,
        materials_of,
        vehicles,
        settings,
        stats,
        ..
    } = params;
    for (index, p) in poses.iter().enumerate() {
        let Ok((_, mut info, mut t, mut cam, mut projection)) = cameras.get_mut(p.camera) else {
            continue;
        };
        let streams = streaming.contains(&p.camera);
        if info.enlarged != streams {
            let size = if streams {
                stream_size(info.base_size)
            } else {
                info.base_size
            };
            if let Some(mut image) = images.get_mut(&info.picture) {
                image.resize(bevy::render::render_resource::Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                });
            }
            info.enlarged = streams;
        }
        let render = streams || picked.contains(&index);
        let live = (count > 0 || streams) && (render || info.last_render != 0);
        for m in &info.members {
            let target = if live { &info.dynamic } else { &m.original };
            if let Ok(mut material) = materials_of.get_mut(m.entity) {
                if material.0 != *target {
                    material.0 = target.clone();
                }
            }
        }
        if cam.is_active != render {
            cam.is_active = render;
        }
        if !render {
            continue;
        }
        info.last_render = frame;
        stats.rendering += 1;
        apply_single_pose(
            p,
            &info,
            &mut t,
            &mut projection,
            settings,
            viewer,
            vehicles,
        );
    }
}
