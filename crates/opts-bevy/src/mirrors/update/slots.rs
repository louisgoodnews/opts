//! The virtual mirror slots: which mirror cameras feed the HUD panels this frame.

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
