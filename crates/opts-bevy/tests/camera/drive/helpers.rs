//! The drive's helpers: the headless app with the vehicle and the camera, the held keys, the frame times and the per-frame measurements.

use std::time::Duration;

use bevy::time::TimeUpdateStrategy;
use opts_bevy::{OptsCameraPlugin, OptsVehiclePlugin, VehicleController};
use opts_sim::vehicle::inputs::GearPosition;

use super::super::*;

/// Frame times between 4 and 25 ms from a fixed sequence (a varying frame rate, reproducible).
pub(super) fn frame_times() -> impl Iterator<Item = f32> {
    let mut state = 0x2545_F491_u32;
    std::iter::repeat_with(move || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        0.004 + 0.021 * (state >> 8) as f32 / (1u32 << 24) as f32
    })
}

/// A headless app with the vehicle physics and the camera, a bus in gear and the driver's view.
pub(super) fn drive_app() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::transform::TransformPlugin,
        bevy::input::InputPlugin,
        OptsVehiclePlugin,
        OptsCameraPlugin,
    ));
    spawn_vehicle(&mut app);
    let vehicle = {
        let mut q = app
            .world_mut()
            .query_filtered::<Entity, With<PlayerVehicle>>();
        q.single(app.world()).unwrap()
    };
    let mut controller = VehicleController::standard_bus(11_500.0, 5.85);
    controller.inputs_mut().set_parking_brake(false);
    controller.inputs_mut().set_gear(GearPosition::Drive);
    app.world_mut()
        .entity_mut(vehicle)
        .insert((controller, Transform::default()));
    let camera = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            opts_bevy::MainCamera,
            Transform::default(),
        ))
        .id();
    app.update();
    // `InputPlugin` clears the keys every frame, so a key tap cannot be used: set the view.
    app.world_mut().resource_mut::<CameraState>().mode = CameraMode::Cockpit;
    app.update();
    assert_eq!(mode(&app), CameraMode::Cockpit);
    (app, vehicle, camera)
}

/// Holds the throttle and swings the wheel right and left every few seconds, so the cab turns and
/// the eye swings around it. (A and D belong to the gear selector; the wheel is on the arrows.)
pub(super) fn hold_keys(app: &mut App, elapsed: f32) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.press(KeyCode::KeyW);
    let phase = (elapsed / 4.0) as u32 % 4;
    for (key, on) in [
        (KeyCode::ArrowRight, phase == 0),
        (KeyCode::ArrowLeft, phase == 2),
    ] {
        if on {
            keys.press(key);
        } else {
            keys.release(key);
        }
    }
}

/// One frame of the drive: the cab's position, its forward direction and the eye's position.
pub(super) struct Frame {
    pub(super) dt: f32,
    pub(super) cab: Vec3,
    pub(super) forward: Vec3,
    pub(super) eye: Vec3,
    pub(super) eye_in_cab: Vec3,
}

/// Runs the drive for `seconds` of frame time and records every frame.
pub(super) fn record(app: &mut App, vehicle: Entity, camera: Entity, seconds: f32) -> Vec<Frame> {
    let (mut frames, mut elapsed) = (Vec::new(), 0.0_f32);
    for dt in frame_times() {
        if elapsed >= seconds {
            break;
        }
        elapsed += dt;
        hold_keys(app, elapsed);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
            dt,
        )));
        app.update();
        let cab = *app.world().get::<GlobalTransform>(vehicle).unwrap();
        let eye = app
            .world()
            .get::<GlobalTransform>(camera)
            .unwrap()
            .translation();
        frames.push(Frame {
            dt,
            cab: cab.translation(),
            forward: cab.forward().as_vec3(),
            eye,
            eye_in_cab: cab.affine().inverse().transform_point3(eye),
        });
    }
    frames
}

/// The most a frame's eye movement differs from the local speed: the mean speed of the frames
/// within half a second either side (the bus accelerates and turns, so the speed is not constant
/// over a drive, but it is smooth). Frames too near either end of the drive are not judged.
pub(super) fn worst_against_local_speed(frames: &[Frame]) -> (f32, usize) {
    // The movement of frame `k` (from the pose of frame `k - 1`) took the frame's own time.
    let moved = |k: usize| (frames[k].cab - frames[k - 1].cab).length();
    let (mut worst, mut judged) = (0.0_f32, 0);
    for i in 1..frames.len() {
        let (mut time, mut distance) = (frames[i].dt, moved(i));
        let (mut back, mut ahead) = (i, i);
        let (mut before, mut after) = (0.0_f32, 0.0_f32);
        while back > 1 && before < 0.5 {
            back -= 1;
            before += frames[back].dt;
            distance += moved(back);
        }
        while ahead + 1 < frames.len() && after < 0.5 {
            ahead += 1;
            after += frames[ahead].dt;
            distance += moved(ahead);
        }
        if before < 0.5 || after < 0.5 {
            continue;
        }
        time += before + after;
        let eye_moved = (frames[i].eye - frames[i - 1].eye).length();
        worst = worst.max((eye_moved - distance / time * frames[i].dt).abs());
        judged += 1;
    }
    (worst, judged)
}
