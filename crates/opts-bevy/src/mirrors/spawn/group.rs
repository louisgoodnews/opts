//! Joining a group's panes to its camera, or creating the camera of a new group.

use bevy::prelude::*;

use crate::mirrors::*;

use super::material::{create_mirror_image, mirror_material, spawn_camera_entity};
use super::scan::Found;
use super::MirrorSpawnParams;

pub(super) fn join_existing_group(
    camera: Entity,
    found: Vec<Found>,
    cameras: &mut Query<(Entity, &mut MirrorCamera)>,
    materials: &mut Assets<StandardMaterial>,
    settings: &MirrorSettings,
    commands: &mut Commands,
) {
    if let Ok((_, mut info)) = cameras.get_mut(camera) {
        for f in found {
            info.uv_min = info.uv_min.min(f.uv_min);
            info.uv_max = info.uv_max.max(f.uv_max);
            commands
                .entity(f.member.entity)
                .insert(MirrorSurface { camera });
            info.members.push(f.member);
        }
        let (dynamic, lo, hi) = (info.dynamic.clone(), info.uv_min, info.uv_max);
        if let Some(mut m) = materials.get_mut(&dynamic) {
            m.uv_transform = if info.tag.is_some() {
                pane_uv_transform(Vec2::ZERO, Vec2::ONE, settings.flip_u, settings.flip_v)
            } else {
                pane_uv_transform(lo, hi, settings.flip_u, settings.flip_v)
            };
        }
    }
}

pub(super) fn create_mirror_camera(
    vehicle: Entity,
    key: GroupKey,
    original: Handle<StandardMaterial>,
    found: Vec<Found>,
    tag: Option<BusMirrorCamera>,
    params: &mut MirrorSpawnParams,
    commands: &mut Commands,
) -> Option<Entity> {
    let base = params.materials.get(&original)?.clone();
    let size = if tag.is_some() {
        let side = ((params.settings.size.x * params.settings.size.y) as f32)
            .sqrt()
            .round() as u32;
        UVec2::splat(side.clamp(32, 1024))
    } else {
        params.settings.size
    };
    let picture = params.images.add(create_mirror_image(size));
    let (uv_min, uv_max) = found.iter().fold(
        (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN)),
        |(lo, hi), f| (lo.min(f.uv_min), hi.max(f.uv_max)),
    );
    let (pic_min, pic_max) = if tag.is_some() {
        (Vec2::ZERO, Vec2::ONE)
    } else {
        (uv_min, uv_max)
    };
    let dynamic = params.materials.add(mirror_material(
        &base,
        picture.clone(),
        pic_min,
        pic_max,
        &params.settings,
    ));
    let aspect = found.iter().map(|f| f.member.aspect).sum::<f32>() / found.len() as f32;
    let aspect_ratio = if tag.is_some() {
        1.0
    } else {
        aspect.clamp(MIRROR_ASPECT_RANGE.0, MIRROR_ASPECT_RANGE.1)
    };
    let members: Vec<MirrorMember> = found.into_iter().map(|f| f.member).collect();
    let camera = spawn_camera_entity(picture.clone(), aspect_ratio, commands);
    for m in &members {
        commands.entity(m.entity).insert(MirrorSurface { camera });
    }
    commands.entity(camera).insert(MirrorCamera {
        vehicle,
        key,
        original,
        dynamic,
        members,
        uv_min,
        uv_max,
        picture,
        base_size: size,
        enlarged: false,
        tag,
        last_render: 0,
    });
    Some(camera)
}
