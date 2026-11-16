use super::*;
use bevy::render::render_resource::{Extent3d, TextureDimension};

#[path = "tests/chain.rs"]
mod chain;
#[path = "tests/fallback.rs"]
mod fallback;

/// A checker of `cell`-texel squares: colour `a` and `b`; `b` transparent when `cutout`.
fn checker(w: usize, h: usize, cell: usize, cutout: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            if (x / cell + y / cell).is_multiple_of(2) {
                out.extend([200, 60, 30, 255]);
            } else if cutout {
                out.extend([0, 0, 0, 0]);
            } else {
                out.extend([20, 120, 220, 255]);
            }
        }
    }
    out
}

fn bc_image(format: TextureFormat, w: u32, h: u32, rgba: &[u8]) -> Image {
    let (tex, _) = block_format(format).expect("bc format");
    let mut data = vec![0u8; tex.compressed_size(w as usize, h as usize)];
    tex.compress(
        rgba,
        w as usize,
        h as usize,
        texpresso::Params::default(),
        &mut data,
    );
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: w,
        height: h,
        depth_or_array_layers: 1,
    };
    image.texture_descriptor.format = format;
    image.texture_descriptor.dimension = TextureDimension::D2;
    image.texture_descriptor.mip_level_count = 1;
    image.data = Some(data);
    image
}

fn levels_of(tex: texpresso::Format, chain: &[u8], w: u32, h: u32, levels: u32) -> Vec<Vec<u8>> {
    let mut offset = 0;
    (0..levels)
        .map(|l| {
            let (lw, lh) = level_size(w, h, l);
            let (lw, lh) = (lw as usize, lh as usize);
            let len = tex.compressed_size(lw, lh);
            let mut rgba = vec![0u8; lw * lh * 4];
            tex.decompress(&chain[offset..offset + len], lw, lh, &mut rgba);
            offset += len;
            rgba
        })
        .collect()
}
