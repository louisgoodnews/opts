//! Tests of the mip-chain generation.

use super::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension};

mod alpha;
mod coverage;
mod layout;

fn image(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Image {
    let mut data = Vec::new();
    for y in 0..h {
        for x in 0..w {
            data.extend_from_slice(&f(x, y));
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

fn level_offset(w: u32, h: u32, level: u32) -> usize {
    (0..level)
        .map(|l| {
            let (lw, lh) = level_size(w, h, l);
            lw as usize * lh as usize * 4
        })
        .sum()
}
