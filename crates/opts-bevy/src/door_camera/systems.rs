//! Systems for detecting open doors, driving the door camera slots, and syncing look from the main camera.

use super::*;

fn leaf_open_door_group(
    anim: &MeshAnimation,
    tf: &GlobalTransform,
    aabb: Option<&Aabb>,
    door_world: &[Vec3],
    doors: &[DoorPoint],
) -> Option<usize> {
    let step = anim
        .steps
        .iter()
        .zip(&anim.shown)
        .find(|(step, value)| is_door_variable(&step.variable) && value.abs() > OPEN_ABOVE)
        .map(|(step, _)| step)?;
    let pivot = step.frame.w_axis.truncate();
    let local = if pivot.length() >= 0.01 {
        Vec3::new(pivot.x, pivot.z, -pivot.y)
    } else {
        Vec3::from(aabb?.center)
    };
    let leaf = tf.transform_point(local);
    let i = nearest_point(door_world, leaf)
        .filter(|&i| (door_world[i].x - leaf.x).hypot(door_world[i].z - leaf.z) < LEAF_NEAR_M)?;
    Some(door_group(doors, i))
}

/// Looks at the door leaves a few times a second and keeps the door that moves.
pub fn detect_open_doors(
    time: Res<Time<Real>>,
    mut last: Local<f64>,
    mut state: ResMut<DoorCamera>,
    vehicles: Query<(Entity, &DoorPoints, Option<&crate::ScriptConsist>), With<PlayerVehicle>>,
    animated: Query<(&MeshAnimation, &GlobalTransform, Option<&Aabb>)>,
    globals: Query<&GlobalTransform>,
) {
    if !state.enabled {
        state.auto = None;
        state.open.clear();
        return;
    }
    let now = time.elapsed_secs_f64();
    if now - *last < DETECT_INTERVAL_S {
        return;
    }
    *last = now;
    let Some((vehicle, doors, consist)) = vehicles.iter().next() else {
        state.auto = None;
        state.open.clear();
        return;
    };
    let mut found = None;
    let mut now_open = Vec::new();
    let door_world: Vec<Vec3> = doors
        .0
        .iter()
        .map(|d| {
            globals
                .get(d.unit.unwrap_or(vehicle))
                .map_or(d.position, |g| g.transform_point(d.position))
        })
        .collect();
    for (anim, tf, aabb) in &animated {
        let ours = anim.owner == vehicle
            || consist.is_some_and(|c| c.units.iter().any(|u| u.part == Some(anim.owner)));
        if !ours {
            continue;
        }
        if let Some(d) = leaf_open_door_group(anim, tf, aabb, &door_world, &doors.0) {
            if !now_open.contains(&d) {
                now_open.push(d);
            }
            found = found.or(Some(d));
        }
    }
    now_open.sort_by_key(|d| state.open.iter().position(|o| o == d).unwrap_or(usize::MAX));
    if state.open != now_open {
        state.open = now_open;
    }
    match found {
        Some(door) => {
            state.auto = Some(door);
            state.auto_until = now + HOLD_S;
        }
        None if now > state.auto_until => state.auto = None,
        None => {}
    }
}

fn spawn_door_slot(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    vehicle: Entity,
    point: &DoorPoint,
    door: usize,
    slot_idx: usize,
) -> DoorSlot {
    let mut image = Image::new_target_texture(
        IMAGE_SIZE.x,
        IMAGE_SIZE.y,
        TextureFormat::Bgra8UnormSrgb,
        None,
    );
    image.sampler = bevy::image::ImageSampler::linear();
    let handle = images.add(image);
    let camera = commands
        .spawn((
            Camera3d::default(),
            Camera {
                order: MIRROR_ORDER - 1 - slot_idx as isize,
                is_active: false,
                ..default()
            },
            RenderTarget::Image(handle.clone().into()),
            Msaa::Off,
            Hdr,
            Tonemapping::None,
            Projection::Perspective(PerspectiveProjection {
                fov: 70f32.to_radians(),
                near: 0.1,
                aspect_ratio: IMAGE_SIZE.x as f32 / IMAGE_SIZE.y as f32,
                ..default()
            }),
            door_pose(point),
            DoorCameraMarker,
            Name::new("Door_Camera"),
        ))
        .id();
    commands
        .entity(point.unit.unwrap_or(vehicle))
        .add_child(camera);
    DoorSlot {
        door,
        vehicle,
        camera,
        image: handle,
        closed_since: None,
        fade: 1.0,
    }
}

/// Keeps the pictures: a slot (camera and image) is made when a door opens, its camera sits at
/// the door and renders at [`RENDER_HZ`]; when the door closes the picture holds for
/// [`FADE_DELAY_S`], fades for [`FADE_S`] and the slot is dropped. Slots of another vehicle go.
pub fn drive_camera(
    time: Res<Time<Real>>,
    mut state: ResMut<DoorCamera>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    vehicles: Query<(Entity, &DoorPoints), With<PlayerVehicle>>,
    mut cameras: Query<(&mut Camera, &mut Transform), With<DoorCameraMarker>>,
) {
    let now = time.elapsed_secs_f64();
    let vehicle = vehicles.iter().next();
    let wanted = state.wanted();
    let mut slots = std::mem::take(&mut state.slots);
    slots.retain(|slot| {
        let keep =
            vehicle.is_some_and(|(v, _)| v == slot.vehicle) && cameras.get(slot.camera).is_ok();
        if !keep {
            commands.entity(slot.camera).try_despawn();
        }
        keep
    });
    for slot in &mut slots {
        if wanted.contains(&slot.door) {
            slot.closed_since = None;
            slot.fade = 1.0;
        } else {
            let since = *slot.closed_since.get_or_insert(now);
            let t = now - since - FADE_DELAY_S;
            slot.fade = (1.0 - t / FADE_S).clamp(0.0, 1.0) as f32;
        }
    }
    slots.retain(|slot| {
        let keep = slot.closed_since.is_none() || slot.fade > 0.0;
        if !keep {
            commands.entity(slot.camera).try_despawn();
        }
        keep
    });
    if let Some((vehicle, doors)) = vehicle {
        for door in wanted {
            if slots.len() >= MAX_SLOTS || slots.iter().any(|s| s.door == door) {
                continue;
            }
            if let Some(point) = doors.0.get(door) {
                slots.push(spawn_door_slot(
                    &mut commands,
                    &mut images,
                    vehicle,
                    point,
                    door,
                    slots.len(),
                ));
            }
        }
    }
    let due = now - state.last_render >= 1.0 / RENDER_HZ;
    for slot in &slots {
        let Ok((mut camera, mut tf)) = cameras.get_mut(slot.camera) else {
            continue;
        };
        if let Some(point) = vehicle.and_then(|(_, d)| d.0.get(slot.door)) {
            let pose = door_pose(point);
            if *tf != pose {
                *tf = pose;
            }
        }
        if camera.is_active != due {
            camera.is_active = due;
        }
    }
    if due && !slots.is_empty() {
        state.last_render = now;
        state.renders += 1;
    }
    state.slots = slots;
}

/// What the picture copies from the main camera: `Hdr` and fog.
type MainLook<'a> = (Has<Hdr>, Option<&'a DistanceFog>, Option<&'a Tonemapping>);
/// The main camera, not a door camera.
type MainFilter = (With<MainCamera>, Without<DoorCameraMarker>);
/// The same look on a door camera, with its entity.
type DoorLook<'a> = (
    Entity,
    Has<Hdr>,
    Option<&'a DistanceFog>,
    Option<&'a Tonemapping>,
);

/// `Hdr`, fog and the tone mapping follow the main camera. The picture is shown as it is on the
/// HUD (nothing tone-maps a UI image), so it carries the main camera's display transform:
/// without it the picture was dark and harsh next to the world.
pub fn sync_look(
    main: Query<MainLook, MainFilter>,
    doors: Query<DoorLook, With<DoorCameraMarker>>,
    mut commands: Commands,
) {
    let Some((hdr, fog, tonemapping)) = main.iter().next() else {
        return;
    };
    for (entity, has_hdr, has_fog, has_tonemapping) in &doors {
        let mut e = commands.entity(entity);
        if let Some(t) = tonemapping {
            if has_tonemapping != Some(t) {
                e.insert(*t);
            }
        }
        if hdr != has_hdr {
            if hdr {
                e.insert(Hdr);
            } else {
                e.remove::<Hdr>();
            }
        }
        match (fog, has_fog) {
            (Some(f), None) => {
                e.insert(f.clone());
            }
            (None, Some(_)) => {
                e.remove::<DistanceFog>();
            }
            _ => {}
        }
    }
}
