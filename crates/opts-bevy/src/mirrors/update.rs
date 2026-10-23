//! Per-frame mirror work: placing the cameras behind the mirrors and following the driver's look.

use super::*;

/// A group's pose in the world for the current frame.
pub(super) struct GroupPose {
    pub(super) camera: Entity,
    pub(super) centre: Vec3,
    pub(super) normal: Vec3,
    pub(super) height: f32,
    pub(super) aspect: f32,
    pub(super) distance: f32,
}

/// Selects, poses and switches the mirror cameras.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_mirror_cameras(
    quality: Res<LightQuality>,
    mut frame: ResMut<MirrorFrame>,
    mut stats: ResMut<MirrorStats>,
    mut commands: Commands,
    settings: Res<MirrorSettings>,
    mut virtual_mirrors: ResMut<VirtualMirrors>,
    mut images: ResMut<Assets<Image>>,
    vehicles: Query<&GlobalTransform, With<PlayerVehicle>>,
    main: Query<&GlobalTransform, (With<MainCamera>, Without<MirrorCamera>)>,
    mut cameras: Query<(
        Entity,
        &mut MirrorCamera,
        &mut Transform,
        &mut Camera,
        &mut Projection,
    )>,
    panes: Query<(&GlobalTransform, Option<&InheritedVisibility>), With<MirrorSurface>>,
    mut materials_of: Query<&mut MeshMaterial3d<StandardMaterial>, With<MirrorSurface>>,
) {
    frame.0 = frame.0.wrapping_add(1);
    // Groups whose panes or vehicle are gone, or all of them when the quality asks for none.
    let mut live = Vec::new();
    for (entity, mut info, ..) in &mut cameras {
        info.members.retain(|m| panes.contains(m.entity));
        let gone = (quality.mirror_count == 0 && !virtual_mirrors.any_stream())
            || !vehicles.contains(info.vehicle)
            || info.members.is_empty();
        if gone {
            for m in &info.members {
                if let Ok(mut e) = commands.get_entity(m.entity) {
                    e.insert(MeshMaterial3d(m.original.clone()))
                        .remove::<MirrorSurface>();
                }
            }
            commands.entity(entity).despawn();
        } else {
            live.push(entity);
        }
    }
    *stats = MirrorStats {
        cameras: live.len(),
        ..default()
    };
    let Some(view) = main.iter().next() else {
        for e in &live {
            if let Ok((_, _, _, mut cam, _)) = cameras.get_mut(*e) {
                if cam.is_active {
                    cam.is_active = false;
                }
            }
        }
        return;
    };
    let (viewer, forward) = (view.translation(), view.forward().as_vec3());

    // World pose of every live group, from its visible panes.
    let mut poses = Vec::with_capacity(live.len());
    for e in &live {
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
    let distances: Vec<f32> = poses.iter().map(|p| p.distance).collect();
    let mut selected = nearest(&distances, usize::from(quality.mirror_count));

    // The virtual mirrors: name the groups by their place on the vehicle and keep the ones that
    // are switched on streaming (always selected, rendered every frame).
    let candidates: Vec<(Entity, Vec3)> = poses
        .iter()
        .filter_map(|p| {
            let info = cameras.get(p.camera).ok()?.1;
            // The tag camera sits where the mirror is, in the vehicle's frame, whether or not
            // its pane is visible (the other variant of a mirror is shown instead).
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
            if let Some(index) = poses.iter().position(|p| p.camera == camera) {
                if !selected.contains(&index) {
                    selected.push(index);
                }
            }
        }
    }
    stats.selected = selected.len();

    for (index, p) in poses.iter().enumerate() {
        let rank = selected.iter().position(|i| *i == index);
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
        // The picture shown on the panes: live when selected, the static texture otherwise.
        for m in &info.members {
            let target = if rank.is_some() {
                &info.dynamic
            } else {
                &m.original
            };
            if let Ok(mut material) = materials_of.get_mut(m.entity) {
                if material.0 != *target {
                    material.0 = target.clone();
                }
            }
        }
        let render = streams
            || rank.is_some_and(|r| {
                mirror_in_front(viewer, forward, p.centre)
                    && is_render_frame(frame.0, r, quality.mirror_every)
            });
        if cam.is_active != render {
            cam.is_active = render;
        }
        if rank.is_none() {
            continue;
        }
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
            if render {
                stats.rendering += 1;
            }
            continue;
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
        if render {
            stats.rendering += 1;
        }
    }
}

/// Copies `Hdr` and `DistanceFog` from the main camera to the mirror cameras (the mirror picture
/// is scene-linear: no tonemapper, no colour grading; the main camera applies its own once).
#[allow(clippy::type_complexity)]
pub fn sync_mirror_camera_look(
    main: Query<(Has<Hdr>, Option<&DistanceFog>), (With<MainCamera>, Without<MirrorCamera>)>,
    mirrors: Query<(Entity, Has<Hdr>, Option<&DistanceFog>), With<MirrorCamera>>,
    mut commands: Commands,
) {
    let Some((hdr, fog)) = main.iter().next() else {
        return;
    };
    for (entity, has_hdr, has_fog) in &mirrors {
        let mut e = commands.entity(entity);
        if hdr && !has_hdr {
            e.insert(Hdr);
        } else if !hdr && has_hdr {
            e.remove::<Hdr>();
        }
        match (fog, has_fog) {
            (Some(f), None) => {
                e.insert(f.clone());
            }
            (Some(f), Some(cur)) if format!("{f:?}") != format!("{cur:?}") => {
                e.insert(f.clone());
            }
            (None, Some(_)) => {
                e.remove::<DistanceFog>();
            }
            _ => {}
        }
    }
}
