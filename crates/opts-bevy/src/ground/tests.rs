//! Unit tests for `super` (`ground.rs`).

use super::*;

fn solid(rgb: [u8; 3], a: u8) -> CpuImage {
    CpuImage {
        width: 2,
        height: 2,
        rgba: [rgb[0], rgb[1], rgb[2], a].repeat(4),
    }
}

#[test]
fn mask_blends_layers_and_row_zero_is_north() {
    let base = solid([0, 0, 255], 255);
    let paint = solid([255, 0, 0], 255);
    // 2x2 mask: opaque only in the top row (north).
    let mask = CpuImage {
        width: 2,
        height: 2,
        rgba: vec![0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0],
    };
    let layers = [
        GroundLayer {
            texture: &base,
            repeats: 1.0,
            mask: None,
        },
        GroundLayer {
            texture: &paint,
            repeats: 1.0,
            mask: Some(&mask),
        },
    ];
    let img = compose_tile_ground(&layers, 4);
    let px = |x: usize, y: usize| &img.data.as_ref().unwrap()[(y * 4 + x) * 4..][..3];
    assert_eq!(px(1, 0), [255, 0, 0], "north half is painted");
    assert_eq!(px(1, 3), [0, 0, 255], "south half keeps the base");
}

#[test]
fn bc1_block_decodes_to_its_endpoint_colours() {
    // c0 = pure red (0xF800), c1 = pure blue (0x001F), indices all 0 -> red.
    let block = [0x00, 0xF8, 0x1F, 0x00, 0, 0, 0, 0];
    let out = decode_bc(&block, 4, 4, 1).expect("decodes");
    assert_eq!(&out[..4], [255, 0, 0, 255]);
}
