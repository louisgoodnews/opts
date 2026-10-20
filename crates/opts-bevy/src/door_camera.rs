//! The door camera (M-0109): a picture-in-picture view from a door of the player's vehicle.
//!
//! * **Doors.** The door points are the pathpoints that the vehicle's passenger cabin names with
//!   `[entry]` and `[exit]` (indices into its `paths.cfg`, vehicle space: x right, y forward, z up),
//!   read once when the vehicle is built (`opts-game`, `vehicle/doors.rs`, which converts them to
//!   Bevy axes) and kept on the vehicle root as [`DoorPoints`]. The point lies on the floor at the door, so the camera sits above it, a little
//!   outside, and looks out and towards the rear ([`door_pose`]).
//! * **Which door.** Two sources, the manual one wins: the key **F4** cycles through the doors and
//!   back to none ([`DoorCamera::cycle`]); the automatic one shows the door whose leaf animates.
//!   Door state variables are named by each vehicle's scripts (`c2_tuer_1_v_pos`, ...), so the
//!   host cannot read them by name; it looks at the **mesh animations** instead: a mesh whose
//!   animation reads a variable that names a door ([`is_door_variable`]) with a shown value away
//!   from zero is an open door leaf, and the door point nearest to it ([`nearest_door`]) is the
//!   door ([`detect_open_doors`], five times a second, held for two seconds after the leaf rests).
//! * **Pictures.** Every open door (at most [`MAX_SLOTS`], in the order they opened) has its own
//!   [`DoorSlot`]: a `Camera3d` child of the vehicle, sitting at the door's upper left (as seen
//!   from outside) and looking into the doorway ([`door_pose`]), rendering into a 480 x 270 image
//!   at 30 Hz ([`RENDER_HZ`]); the HUD stacks the images from the top right downwards. A door that
//!   closes keeps its picture [`FADE_DELAY_S`], fades it out over [`FADE_S`] and then drops the
//!   slot, so the pictures below move up. Like the mirror cameras they render scene-linear (no
//!   tonemapping) and copy `Hdr` and fog from the [`MainCamera`].
//! * **Setting.** `camera.door_overlay` (off by default) sets [`DoorCamera::enabled`]; while it is
//!   off nothing exists: no camera, no image, no scan.

use bevy::camera::{Hdr, RenderTarget};
use bevy::pbr::DistanceFog;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::Tonemapping;

use crate::mirrors::MIRROR_ORDER;
use crate::vehicle_anim::MeshAnimation;
use crate::vehicle_sim::PlayerVehicle;
use crate::MainCamera;

/// Size of the picture (the HUD shows it as it is).
pub const IMAGE_SIZE: UVec2 = UVec2::new(480, 270);
/// How often the picture is rendered.
pub const RENDER_HZ: f64 = 30.0;
/// Height of the camera above the floor point of the door (m).
pub const EYE_HEIGHT_M: f32 = 2.1;
/// How far outside the door point the camera sits (m).
pub const EYE_OUT_M: f32 = 0.6;
/// How far to the upper left of the door (along the vehicle) the camera sits (m).
pub const EYE_ALONG_M: f32 = 0.9;
/// Seconds between two looks at the animated meshes.
pub const DETECT_INTERVAL_S: f64 = 0.2;
/// Seconds a door stays shown after its leaf stopped moving.
pub const HOLD_S: f64 = 2.0;
/// Pictures shown at once.
pub const MAX_SLOTS: usize = 4;
/// Seconds a closed door's picture stays before it fades.
pub const FADE_DELAY_S: f64 = 2.0;
/// Seconds the fade takes.
pub const FADE_S: f64 = 3.0;
/// A leaf is at most this far from its door point in the floor plane (m).
pub const LEAF_NEAR_M: f32 = 2.5;
/// A door animation value above this counts as open.
pub const OPEN_ABOVE: f32 = 0.05;

/// Which end of the vehicle a door is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// The forward third of the doors' extent.
    Front,
    /// Between.
    Middle,
    /// The rear third.
    Rear,
}

/// One door of the vehicle.
#[derive(Debug, Clone, PartialEq)]
pub struct DoorPoint {
    /// The pathpoint is an `[entry]` (else an `[exit]`).
    pub entry: bool,
    /// The pathpoint index in `paths.cfg`.
    pub index: usize,
    /// Floor position at the door, in the vehicle's frame (Bevy axes).
    pub position: Vec3,
    /// `1` for a door on the right side, `-1` on the left.
    pub side: i8,
}

/// The doors of a vehicle (on its root).
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct DoorPoints(pub Vec<DoorPoint>);

impl DoorPoints {
    /// Where the door is along the vehicle compared to the other doors.
    #[must_use]
    pub fn zone(&self, door: usize) -> Zone {
        let Some(d) = self.0.get(door) else {
            return Zone::Middle;
        };
        // Bevy: forward is -Z, so the smaller z is further front.
        let (front, rear) = self.0.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.position.z), hi.max(p.position.z))
        });
        let span = rear - front;
        if span < 1.0 {
            return Zone::Middle;
        }
        let t = (d.position.z - front) / span;
        if t < 1.0 / 3.0 {
            Zone::Front
        } else if t > 2.0 / 3.0 {
            Zone::Rear
        } else {
            Zone::Middle
        }
    }
}

/// The pose of a door camera in the vehicle's frame: stuck to the door, above its upper left
/// corner as seen from outside (towards the rear for a door on the right, towards the front on
/// the left), a little outside, looking down into the doorway.
#[must_use]
pub fn door_pose(door: &DoorPoint) -> Transform {
    let side = f32::from(door.side);
    // Bevy: x right, y up, z backward.
    let eye = door.position + Vec3::new(side * EYE_OUT_M, EYE_HEIGHT_M, side * EYE_ALONG_M);
    let target = door.position + Vec3::new(-side * 0.2, 0.7, 0.0);
    Transform::from_translation(eye).looking_at(target, Vec3::Y)
}

/// Whether a variable name is the position of a door leaf: it names a door and none of the
/// auxiliaries (alpha, warnings, speeds, triggers, sounds, requests, timers, ...).
#[must_use]
pub fn is_door_variable(name: &str) -> bool {
    let n = name.to_lowercase();
    let door = n.contains("door") || n.contains("tuer") || n.contains("tür");
    const AUX: [&str; 15] = [
        "taster",
        "alpha",
        "warn",
        "speed",
        "geschwindigkeit",
        "trigger",
        "sound",
        "req",
        "timer",
        "light",
        "bt_",
        "cp_",
        "hit",
        "stop",
        "free",
    ];
    door && !AUX.iter().any(|a| n.contains(a))
}

/// The door nearest to `leaf` (a point in the vehicle's frame), by distance in the floor plane.
#[must_use]
pub fn nearest_door(doors: &[DoorPoint], leaf: Vec3) -> Option<usize> {
    doors
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            let da = (a.position.x - leaf.x).hypot(a.position.z - leaf.z);
            let db = (b.position.x - leaf.x).hypot(b.position.z - leaf.z);
            da.total_cmp(&db)
        })
        .map(|(i, _)| i)
}

/// One picture: a door with its camera and image.
#[derive(Debug, Clone)]
pub struct DoorSlot {
    /// Index into the vehicle's [`DoorPoints`].
    pub door: usize,
    /// The vehicle and the camera that renders for it.
    pub vehicle: Entity,
    /// The camera entity.
    pub camera: Entity,
    /// The picture.
    pub image: Handle<Image>,
    /// Real time the door was seen closed (the fade starts [`FADE_DELAY_S`] later).
    pub closed_since: Option<f64>,
    /// Opacity of the picture, `1` to `0`.
    pub fade: f32,
}

/// Door points closer than this along the vehicle (m), on the same side, are one door (the
/// two lanes of an entrance, or the two leaves).
pub const SAME_DOOR_M: f32 = 1.6;

/// The first point of the door that `door` belongs to: the points of one entrance or exit lie
/// within [`SAME_DOOR_M`] of each other on the same side, and one opening shows one picture.
#[must_use]
pub fn door_group(doors: &[DoorPoint], door: usize) -> usize {
    let Some(d) = doors.get(door) else {
        return door;
    };
    doors
        .iter()
        .position(|o| o.side == d.side && (o.position.z - d.position.z).abs() < SAME_DOOR_M)
        .unwrap_or(door)
}

/// What the door camera shows and did.
#[derive(Resource, Debug, Clone, Default)]
pub struct DoorCamera {
    /// The setting `camera.door_overlay`.
    pub enabled: bool,
    /// The door chosen with the key (wins over the automatic one).
    pub manual: Option<usize>,
    /// The door whose leaf moves (or moved a moment ago).
    pub auto: Option<usize>,
    /// Real time until which `auto` is held.
    pub auto_until: f64,
    /// Every door whose leaf is open now (not held).
    pub open: Vec<usize>,
    /// The pictures, in the order the doors opened.
    pub slots: Vec<DoorSlot>,
    /// Real time of the last render request.
    pub last_render: f64,
    /// Pictures rendered since the session started.
    pub renders: u64,
}

impl DoorCamera {
    /// The door being shown first (`None`: nothing, and the cameras are off).
    #[must_use]
    pub fn shown(&self) -> Option<usize> {
        if self.enabled {
            self.manual.or(self.auto)
        } else {
            None
        }
    }

    /// The doors that should have a picture now: the manual one and the open ones.
    #[must_use]
    pub fn wanted(&self) -> Vec<usize> {
        if !self.enabled {
            return Vec::new();
        }
        let mut w: Vec<usize> = self.manual.into_iter().collect();
        for d in &self.open {
            if !w.contains(d) {
                w.push(*d);
            }
        }
        w
    }

    /// F4: the next door, after the last none again.
    pub fn cycle(&mut self, doors: usize) {
        self.manual = match self.manual {
            None if doors > 0 => Some(0),
            Some(i) if i + 1 < doors => Some(i + 1),
            _ => None,
        };
    }
}

/// Marks the door camera entity.
#[derive(Component, Debug, Clone, Copy)]
pub struct DoorCameraMarker;

/// F4 cycles through the doors.
pub fn door_keys(
    keys: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<crate::action_keys::ActionKeys>>,
    vehicles: Query<&DoorPoints, With<PlayerVehicle>>,
    mut state: ResMut<DoorCamera>,
) {
    if !state.enabled
        || !crate::action_keys::resolve(&action_keys).just_pressed(&keys, "door_camera")
    {
        return;
    }
    let doors = vehicles.iter().next().map_or(0, |d| d.0.len());
    state.cycle(doors);
}

/// Looks at the door leaves a few times a second and keeps the door that moves.
pub fn detect_open_doors(
    time: Res<Time<Real>>,
    mut last: Local<f64>,
    mut state: ResMut<DoorCamera>,
    vehicles: Query<(Entity, &DoorPoints, &GlobalTransform), With<PlayerVehicle>>,
    animated: Query<(&MeshAnimation, &GlobalTransform)>,
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
    let Some((vehicle, doors, _)) = vehicles.iter().next() else {
        state.auto = None;
        state.open.clear();
        return;
    };
    let mut found = None;
    let mut now_open = Vec::new();
    for (anim, _tf) in &animated {
        if anim.owner != vehicle {
            continue;
        }
        // The pivot of the first step that moves a door variable is where the leaf hangs (the
        // mesh entity's own origin is the vehicle's); vehicle axes (x right, y forward, z up)
        // to Bevy's (x right, y up, z backward).
        let Some(step) = anim
            .steps
            .iter()
            .zip(&anim.shown)
            .find(|(step, value)| is_door_variable(&step.variable) && value.abs() > OPEN_ABOVE)
            .map(|(step, _)| step)
        else {
            continue;
        };
        let pivot = step.frame.w_axis.truncate();
        if pivot.length() < 0.01 {
            continue;
        }
        let local = Vec3::new(pivot.x, pivot.z, -pivot.y);
        // Only a leaf near a door point counts.
        if !doors
            .0
            .iter()
            .any(|d| (d.position.x - local.x).hypot(d.position.z - local.z) < LEAF_NEAR_M)
        {
            continue;
        }
        if let Some(d) = nearest_door(&doors.0, local).map(|d| door_group(&doors.0, d)) {
            if !now_open.contains(&d) {
                now_open.push(d);
            }
            found = found.or(Some(d));
        }
    }
    // Doors keep the order they opened in.
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

/// Keeps the pictures: a slot (camera and image) is made when a door opens, its camera sits at
/// the door and renders at [`RENDER_HZ`]; when the door closes the picture holds for
/// [`FADE_DELAY_S`], fades for [`FADE_S`] and the slot is dropped. Slots of another vehicle go.
#[allow(clippy::too_many_arguments)]
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
    // Slots of another vehicle, or whose camera is gone, are dropped.
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
            let Some(point) = doors.0.get(door) else {
                continue;
            };
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
                        order: MIRROR_ORDER - 1 - slots.len() as isize,
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
            commands.entity(vehicle).add_child(camera);
            slots.push(DoorSlot {
                door,
                vehicle,
                camera,
                image: handle,
                closed_since: None,
                fade: 1.0,
            });
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
type MainLook<'a> = (Has<Hdr>, Option<&'a DistanceFog>);
/// The main camera, not a door camera.
type MainFilter = (With<MainCamera>, Without<DoorCameraMarker>);
/// The same look on a door camera, with its entity.
type DoorLook<'a> = (Entity, Has<Hdr>, Option<&'a DistanceFog>);

/// The picture is scene-linear like the mirrors': `Hdr` and fog follow the main camera.
pub fn sync_look(
    main: Query<MainLook, MainFilter>,
    doors: Query<DoorLook, With<DoorCameraMarker>>,
    mut commands: Commands,
) {
    let Some((hdr, fog)) = main.iter().next() else {
        return;
    };
    for (entity, has_hdr, has_fog) in &doors {
        let mut e = commands.entity(entity);
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

/// Registers the door camera.
pub struct DoorCameraPlugin;

impl Plugin for DoorCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DoorCamera>().add_systems(
            Update,
            (
                door_keys.run_if(crate::reporting::not_modal_active),
                detect_open_doors,
                drive_camera.run_if(resource_exists::<Assets<Image>>),
                sync_look,
            )
                .chain(),
        );
    }
}

#[cfg(test)]
mod tests;
