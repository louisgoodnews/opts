//! The door pictures: a slot (camera and image) per open door, and the fade and drop of closed ones.

use super::*;

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
