//! Tests of the capture encoder.

use super::*;

fn frame(w: u32, h: u32) -> RgbFrame {
    RgbFrame {
        width: w,
        height: h,
        data: (0..w * h * 3).map(|i| (i % 251) as u8).collect(),
    }
}

#[test]
fn box_reduction_averages_blocks() {
    // 2x2 pixels: (0,0,0) (100,0,0) (0,200,0) (100,200,60) -> one pixel (50,100,15).
    let data = vec![0, 0, 0, 100, 0, 0, 0, 200, 0, 100, 200, 60];
    let f = RgbFrame::box_reduce(2, 2, data, 2);
    assert_eq!((f.width, f.height, f.data), (1, 1, vec![50, 100, 15]));
    let g = RgbFrame::box_reduce(7, 5, vec![9; 7 * 5 * 3], 3);
    assert_eq!((g.width, g.height), (2, 1));
}

#[test]
fn fitting_keeps_or_resizes() {
    assert_eq!(frame(8, 6).fitted(8, 6).data.len(), 8 * 6 * 3);
    let f = frame(8, 6).fitted(4, 4);
    assert_eq!((f.width, f.height, f.data.len()), (4, 4, 48));
}

#[test]
fn png_sequence_writes_numbered_files() {
    let dir = std::env::temp_dir().join(format!("opts_pngseq_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut enc = PngSequence::create(&dir).expect("create");
    let info = FrameInfo {
        index: 0,
        sim_time: 0.0,
        frame: 0,
        wall_ms: 0,
    };
    enc.push(&frame(16, 12), &info).expect("push");
    enc.push(&frame(16, 12), &info).expect("push");
    assert_eq!(
        image::open(dir.join("000002.png")).expect("png").width(),
        16
    );
    assert!(Box::new(enc).finish().expect("finish") > 0);
}
