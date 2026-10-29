//! The driver's view over a long drive with real vehicle physics and a varying frame rate
//! (M-0109): the camera must sit where the cab is, in the frame the cab moved, and its
//! per-frame movement must follow the vehicle's speed within 2 mm.
//!
//! Measured 2026-10-29 (60 s, 4,184 frames of 4 to 25 ms, 1,231 m driven, 6,487 degrees of
//! turning): eye placement error 0.000 mm, eye against cab 0.000 mm per frame, per-frame
//! movement against the local speed 1.9 mm at most.

use std::time::Duration;

use bevy::time::TimeUpdateStrategy;
use opts_bevy::{OptsCameraPlugin, OptsVehiclePlugin, VehicleController};
use opts_sim::vehicle::inputs::GearPosition;

use super::*;

/// Frame times between 4 and 25 ms from a fixed sequence (a varying frame rate, reproducible).
fn frame_times() -> impl Iterator<Item = f32> {
    let mut state = 0x2545_F491_u32;
    std::iter::repeat_with(move || {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        0.004 + 0.021 * (state >> 8) as f32 / (1u32 << 24) as f32
    })
}

/// A headless app with the vehicle physics and the camera, a bus in gear and the driver's view.
fn drive_app() -> (App, Entity, Entity) {
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
fn hold_keys(app: &mut App, elapsed: f32) {
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
struct Frame {
    dt: f32,
    cab: Vec3,
    forward: Vec3,
    eye: Vec3,
    eye_in_cab: Vec3,
}

/// Runs the drive for `seconds` of frame time and records every frame.
fn record(app: &mut App, vehicle: Entity, camera: Entity, seconds: f32) -> Vec<Frame> {
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
fn worst_against_local_speed(frames: &[Frame]) -> (f32, usize) {
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

#[test]
fn a_60_second_drive_keeps_the_driver_view_with_the_cab_within_2_mm() {
    let (mut app, vehicle, camera) = drive_app();
    let frames = record(&mut app, vehicle, camera, 60.0);

    // The bus must really have driven and turned, or the measurement proves nothing.
    let driven: f32 = frames
        .windows(2)
        .map(|w| (w[1].cab - w[0].cab).length())
        .sum();
    let turned: f32 = frames
        .windows(2)
        .map(|w| w[0].forward.angle_between(w[1].forward))
        .sum();
    assert!(driven > 300.0, "the bus must really drive: {driven} m");
    assert!(turned > 0.3, "the bus must really turn: {turned} rad");

    // The eye is where the cab is, in the frame the cab moved (the same-frame placement).
    let first = frames[0].eye_in_cab;
    let placement = frames
        .iter()
        .map(|f| (f.eye_in_cab - first).length())
        .fold(0.0, f32::max);
    let step = frames
        .windows(2)
        .map(|w| ((w[1].eye - w[0].eye).length() - (w[1].cab - w[0].cab).length()).abs())
        .fold(0.0, f32::max);
    let (stairs, judged) = worst_against_local_speed(&frames);
    eprintln!(
        "{} frames, {driven:.0} m driven, {:.0} deg turned: eye placement error {:.3} mm, eye against cab per frame {:.3} mm, per-frame movement against the local speed {:.1} mm over {judged} frames",
        frames.len(),
        turned.to_degrees(),
        placement * 1000.0,
        step * 1000.0,
        stairs * 1000.0
    );
    assert!(placement < 1e-4, "eye placement drifted by {placement} m");
    assert!(
        step < 1e-4,
        "eye and cab moved differently by {step} m in a frame"
    );
    // M-0109 criterion 3: the per-frame movement follows the vehicle's speed within 2 mm, so the
    // 64 Hz physics step does not show as judder at an irregular frame rate and the view needs
    // no extra smoothing.
    assert!(
        stairs < 2e-3,
        "the view stair-steps by {stairs} m in a frame"
    );
}
