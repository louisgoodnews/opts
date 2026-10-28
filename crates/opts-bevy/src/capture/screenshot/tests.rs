//! Unit tests for `super` (`screenshot.rs`).

use super::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use opts_cli::protocol::code;

fn frame(w: u32, h: u32) -> Image {
    let mut data = Vec::new();
    for y in 0..h {
        for x in 0..w {
            data.extend_from_slice(&[(x * 255 / w) as u8, (y * 255 / h) as u8, 80, 255]);
        }
    }
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn temp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("opts_capshot_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("temp dir");
    p
}

fn options(format: StillFormat) -> Options {
    Options {
        target: Target::Generated,
        format,
        quality: 80,
        include_metadata: true,
    }
}

#[test]
fn a_png_has_the_frame_dimensions_and_a_sidecar_with_sim_time_and_frame() {
    let dir = temp("png");
    let path = dir.join("shot.png");
    let mut meta = Map::new();
    meta.insert("sim_time".into(), json!(12.5));
    meta.insert("frame".into(), json!(900));
    let reply =
        finish(frame(64, 48), &path, &options(StillFormat::Png), Some(meta)).expect("finish");
    let png = image::open(&path).expect("decode");
    assert_eq!((png.width(), png.height()), (64, 48));
    assert_eq!(
        (reply["width"].as_u64(), reply["height"].as_u64()),
        (Some(64), Some(48))
    );
    let side: Value = serde_json::from_slice(&std::fs::read(sidecar_path(&path)).expect("sidecar"))
        .expect("json");
    assert_eq!(side["sim_time"], 12.5);
    assert_eq!(side["frame"], 900);
    assert_eq!(side["width"], 64);
    assert_eq!(reply["metadata"]["frame"], 900);
}

#[test]
fn jpeg_and_no_metadata() {
    let dir = temp("jpg");
    let path = dir.join("shot.jpg");
    let reply = finish(frame(32, 32), &path, &options(StillFormat::Jpeg), None).expect("finish");
    assert_eq!(image::open(&path).expect("decode").width(), 32);
    assert!(reply.get("metadata").is_none());
    assert!(!sidecar_path(&path).exists());
}

#[test]
fn parameters_are_validated() {
    use serde_json::json;
    assert!(parse_options(&json!({})).is_ok());
    let both = parse_options(&json!({"path": "/x/a.png", "name": "a.png"})).unwrap_err();
    assert_eq!(both.code, code::INVALID_PARAMS);
    assert_eq!(
        parse_options(&json!({"format": "bmp"})).unwrap_err().code,
        code::INVALID_PARAMS
    );
    assert_eq!(
        parse_options(&json!({"quality": 0})).unwrap_err().code,
        code::INVALID_PARAMS
    );
    let o = parse_options(&json!({"name": "a/b.png", "format": "jpeg", "include_metadata": false}))
        .expect("ok");
    assert_eq!(o.target, Target::Name("a/b.png".into()));
    assert!(!o.include_metadata);
}
