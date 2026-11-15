//! AV1 recordings: a well-formed IVF that stops at the duration limit.

use super::*;

#[test]
fn an_av1_recording_is_a_well_formed_ivf_that_stops_at_the_duration_limit() {
    let root = temp("av1");
    let (mut app, port) = app(&root, deliver);
    let (start, status) = run(&mut app, port, |c| {
        let start = c
            .call(
                "capture.record.start",
                json!({"name": "clip", "fps": 30, "duration_ms": 600, "scale": 1.0}),
            )
            .expect("start");
        let id = start["recording_id"].as_str().expect("id").to_string();
        let status = wait_finished(c, &id);
        (start, status)
    });
    assert_eq!(status["state"], "finished");
    assert_eq!(status["stopped_by"], "duration");
    let took = status["duration_ms"].as_u64().expect("duration");
    assert!(
        (600..=1600).contains(&took),
        "stops within 1 s of the limit, took {took} ms"
    );
    let path = PathBuf::from(start["path"].as_str().expect("path"));
    let bytes = std::fs::read(&path).expect("ivf");
    let info = ivf::inspect(&bytes).expect("well-formed ivf");
    let frames = status["frame_count"].as_u64().expect("frames") as u32;
    assert!(frames >= 8, "got {frames} frames");
    assert_eq!((info.fourcc, info.width, info.height), (*b"AV01", 64, 48));
    assert_eq!((info.header_frames, info.counted_frames), (frames, frames));
    let side: Value = serde_json::from_slice(
        &std::fs::read(format!("{}.json", path.display())).expect("sidecar"),
    )
    .expect("json");
    assert_eq!(
        side["frames"].as_array().map(Vec::len),
        Some(frames as usize)
    );
    assert!(side["frames"][0]["sim_time"].is_number());
    if let Ok(out) = std::process::Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-count_packets",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=codec_name,nb_read_packets",
            "-of",
            "csv=p=0",
        ])
        .arg(&path)
        .output()
    {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            assert_eq!(
                text,
                format!("av1,{frames}"),
                "ffprobe agrees with the header"
            );
        }
    }
    // The documented remux path (`ffmpeg -c copy`) works on our own writer's output.
    for ext in ["mp4", "mkv", "webm"] {
        let out_path = root.join(format!("remux.{ext}"));
        let remux = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(&path)
            .args(["-c", "copy"])
            .arg(&out_path)
            .status();
        if let Ok(status) = remux {
            assert!(status.success(), "ffmpeg -c copy to {ext}");
            assert!(std::fs::metadata(&out_path)
                .map(|m| m.len() > 0)
                .unwrap_or(false));
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}

