//! The driver's view over a long drive with real vehicle physics and a varying frame rate
//! (M-0109): the camera must sit where the cab is, in the frame the cab moved, and its
//! per-frame movement must follow the vehicle's speed within 2 mm.
//!
//! Measured 2026-10-29 (60 s, 4,184 frames of 4 to 25 ms, 1,231 m driven, 6,487 degrees of
//! turning): eye placement error 0.000 mm, eye against cab 0.000 mm per frame, per-frame
//! movement against the local speed 1.9 mm at most.

#[path = "drive/helpers.rs"]
mod helpers;

use helpers::{drive_app, record, worst_against_local_speed};

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
