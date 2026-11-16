//! Tests of the AV1 recorder: quantizer mapping, presentation time gaps, and a well-formed IVF.

use super::*;
use crate::capture::ivf;

fn gradient(w: u32, h: u32, t: u32) -> RgbFrame {
    let mut data = Vec::new();
    for y in 0..h {
        for x in 0..w {
            data.extend_from_slice(&[((x + t * 4) % 256) as u8, (y * 255 / h) as u8, 90]);
        }
    }
    RgbFrame {
        width: w,
        height: h,
        data,
    }
}

#[test]
fn dropped_frames_leave_a_gap_in_presentation_time() {
    let path = std::env::temp_dir().join(format!("opts_av1_vfr_{}.ivf", std::process::id()));
    let mut enc = Av1Ivf::create(&path, 64, 48, 30, 60).expect("create");
    // 30 fps clock; frames 1 and 2 were dropped, so the third arrives at 100 ms.
    for (i, wall_ms) in [0u64, 33, 133, 166].into_iter().enumerate() {
        let info = FrameInfo {
            index: i as u64,
            sim_time: 0.0,
            frame: 0,
            wall_ms,
        };
        enc.push(&gradient(64, 48, i as u32), &info).expect("push");
    }
    Box::new(enc).finish().expect("finish");
    let i = ivf::inspect(&std::fs::read(&path).expect("read")).expect("ivf");
    assert_eq!(i.counted_frames, 4);
    assert_eq!(
        i.last_pts, 5,
        "166 ms at 30 fps is tick 5, not frame number 3"
    );
}

#[test]
fn quantizer_follows_quality() {
    assert!(quantizer_for(100) < quantizer_for(50));
    assert!(quantizer_for(50) < quantizer_for(1));
}

#[test]
fn white_and_black_map_to_the_limited_range() {
    let white = RgbFrame {
        width: 2,
        height: 2,
        data: vec![255; 12],
    };
    let black = RgbFrame {
        width: 2,
        height: 2,
        data: vec![0; 12],
    };
    assert_eq!(to_yuv420(&white)[0][0], 235);
    assert_eq!(to_yuv420(&black)[0][0], 16);
    assert_eq!(to_yuv420(&white)[1][0], 128);
}

#[test]
fn a_short_clip_is_a_well_formed_ivf_with_every_frame() {
    let path = std::env::temp_dir().join(format!("opts_av1_{}.ivf", std::process::id()));
    let mut enc = Av1Ivf::create(&path, 160, 90, 30, 60).expect("create");
    let info = FrameInfo {
        index: 0,
        sim_time: 0.0,
        frame: 0,
        wall_ms: 0,
    };
    for t in 0..6 {
        enc.push(&gradient(160, 90, t), &info).expect("push");
    }
    Box::new(enc).finish().expect("finish");
    let bytes = std::fs::read(&path).expect("read");
    let i = ivf::inspect(&bytes).expect("ivf");
    assert_eq!((i.fourcc, i.width, i.height), (*b"AV01", 160, 90));
    assert_eq!(i.header_frames, 6);
    assert_eq!(i.counted_frames, 6);
}
