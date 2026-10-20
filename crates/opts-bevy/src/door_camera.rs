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
//! * **Picture.** One `Camera3d` child of the vehicle renders into a 480 x 270 image at 30 Hz
//!   ([`RENDER_HZ`]) and only while a door is shown (`Camera::is_active` is the switch); the HUD
//!   shows the image. Like the mirror cameras it renders scene-linear (no tonemapping) and copies
//!   `Hdr` and fog from the [`MainCamera`].
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
pub const EYE_HEIGHT_M: f32 = 1.5;
/// How far outside the door point the camera sits (m).
pub const EYE_OUT_M: f32 = 0.3;
/// Seconds between two looks at the animated meshes.
pub const DETECT_INTERVAL_S: f64 = 0.2;
/// Seconds a door stays shown after its leaf stopped moving.
pub const HOLD_S: f64 = 2.0;
/// Seconds without a shown door after which the camera entity is removed.
pub const REMOVE_AFTER_S: f64 = 5.0;
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

/// The pose of the door camera in the vehicle's frame: above the door point, a little outside,
/// looking out and towards the rear and slightly down, so the kerb and the passengers show.
#[must_use]
pub fn door_pose(door: &DoorPoint) -> Transform {
    let out = Vec3::X * f32::from(door.side);
    let eye = door.position + Vec3::Y * EYE_HEIGHT_M + out * EYE_OUT_M;
    let look = out + Vec3::Z * 0.5 - Vec3::Y * 0.25;
    Transform::from_translation(eye).looking_to(look, Vec3::Y)
}

/// Whether a variable name is the position of a door leaf: it names a door and none of the
/// auxiliaries (alpha, warnings, speeds, triggers, sounds, requests, timers, ...).
#[must_use]
pub fn is_door_variable(name: &str) -> bool {
    let n = name.to_lowercase();
    let door = n.contains("door") || n.contains("tuer") || n.contains("tür");
    const AUX: [&str; 14] = [
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
    /// The picture.
    pub image: Option<Handle<Image>>,
    /// The vehicle and the camera that renders for it.
    pub camera: Option<(Entity, Entity)>,
    /// Real time of the last render request.
    pub last_render: f64,
    /// Since when no door has been shown (the camera is removed after [`REMOVE_AFTER_S`]).
    pub hidden_since: Option<f64>,
    /// Pictures rendered since the session started.
    pub renders: u64,
}

impl DoorCamera {
    /// The door being shown (`None`: nothing, and the camera is off).
    #[must_use]
    pub fn shown(&self) -> Option<usize> {
        if self.enabled {
            self.manual.or(self.auto)
        } else {
            None
        }
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
    vehicles: Query<&DoorPoints, With<PlayerVehicle>>,
    mut state: ResMut<DoorCamera>,
) {
    if !state.enabled || !keys.just_pressed(KeyCode::F4) {
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
        return;
    }
    let now = time.elapsed_secs_f64();
    if now - *last < DETECT_INTERVAL_S {
        return;
    }
    *last = now;
    let Some((vehicle, doors, vehicle_tf)) = vehicles.iter().next() else {
        state.auto = None;
        return;
    };
    let to_local = vehicle_tf.affine().inverse();
    let mut found = None;
    for (anim, tf) in &animated {
        if anim.owner != vehicle {
            continue;
        }
        let open = anim
            .steps
            .iter()
            .zip(&anim.shown)
            .any(|(step, value)| is_door_variable(&step.variable) && value.abs() > OPEN_ABOVE);
        if open {
            let local = to_local.transform_point3(tf.translation());
            found = nearest_door(&doors.0, local);
            break;
        }
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

/// Keeps the camera: made when a door is first shown for a vehicle, posed at the shown door and
/// rendering at [`RENDER_HZ`] only while one is shown; gone with its vehicle.
#[allow(clippy::too_many_arguments)]
pub fn drive_camera(
    time: Res<Time<Real>>,
    mut state: ResMut<DoorCamera>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    vehicles: Query<(Entity, &DoorPoints), With<PlayerVehicle>>,
    mut cameras: Query<(&mut Camera, &mut Transform), With<DoorCameraMarker>>,
) {
    let vehicle = vehicles.iter().next();
    // A camera of another vehicle (the player switched) is removed.
    if let Some((owner, cam)) = state.camera {
        if vehicle.is_none_or(|(v, _)| v != owner) || cameras.get(cam).is_err() {
            commands.entity(cam).try_despawn();
            state.camera = None;
        }
    }
    let (Some((vehicle, doors)), Some(door)) = (vehicle, state.shown()) else {
        if let Some((_, cam)) = state.camera {
            if let Ok((mut c, _)) = cameras.get_mut(cam) {
                c.is_active = false;
            }
            // Nothing is shown: the camera stays a moment (the next F4 press is quick), then goes
            // and costs nothing.
            let now = time.elapsed_secs_f64();
            let since = *state.hidden_since.get_or_insert(now);
            if now - since > REMOVE_AFTER_S {
                commands.entity(cam).try_despawn();
                state.camera = None;
                state.hidden_since = None;
            }
        }
        return;
    };
    state.hidden_since = None;
    let Some(point) = doors.0.get(door) else {
        return;
    };
    if state.camera.is_none() {
        let handle = state.image.clone().unwrap_or_else(|| {
            let mut image = Image::new_target_texture(
                IMAGE_SIZE.x,
                IMAGE_SIZE.y,
                TextureFormat::Bgra8UnormSrgb,
                None,
            );
            image.sampler = bevy::image::ImageSampler::linear();
            images.add(image)
        });
        state.image = Some(handle.clone());
        let cam = commands
            .spawn((
                Camera3d::default(),
                Camera {
                    order: MIRROR_ORDER - 1,
                    is_active: false,
                    ..default()
                },
                RenderTarget::Image(handle.into()),
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
        commands.entity(vehicle).add_child(cam);
        state.camera = Some((vehicle, cam));
        return;
    }
    let Some((_, cam)) = state.camera else {
        return;
    };
    let now = time.elapsed_secs_f64();
    let due = now - state.last_render >= 1.0 / RENDER_HZ;
    if let Ok((mut camera, mut tf)) = cameras.get_mut(cam) {
        let pose = door_pose(point);
        if *tf != pose {
            *tf = pose;
        }
        if camera.is_active != due {
            camera.is_active = due;
        }
    }
    if due {
        state.last_render = now;
        state.renders += 1;
    }
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
