//! Unit tests for `super` (`ground_material.rs`).

use super::*;

fn solid(v: u8) -> CpuImage {
    CpuImage {
        width: 4,
        height: 4,
        rgba: [v, v, v, 255].repeat(16),
    }
}

#[test]
fn layer_array_holds_a_full_mip_chain_per_layer() {
    let (a, b) = (solid(10), solid(200));
    let img = layer_array(&[&a, &b], 8);
    // 8x8 + 4x4 + 2x2 + 1x1 texels per layer, RGBA8.
    let per_layer = (64 + 16 + 4 + 1) * 4;
    assert_eq!(img.data.as_ref().unwrap().len(), 2 * per_layer);
    assert_eq!(img.texture_descriptor.mip_level_count, 4);
    assert_eq!(
        img.data.as_ref().unwrap()[per_layer],
        200,
        "second layer starts after the first layer's mips"
    );
}

#[test]
fn mask_array_defaults_base_on_and_missing_layers_off() {
    let m = CpuImage {
        width: 2,
        height: 2,
        rgba: [0, 0, 0, 128].repeat(4),
    };
    let img = mask_array(&[None, Some(&m), None], 2);
    assert_eq!(&img.data.as_ref().unwrap()[0..4], &[255; 4]);
    assert_eq!(&img.data.as_ref().unwrap()[4..8], &[128; 4]);
    assert_eq!(&img.data.as_ref().unwrap()[8..12], &[0; 4]);
}

#[test]
fn slots_beyond_the_eighth_reach_the_shader_uniforms() {
    let slots: Vec<GroundSlot> = (0..19)
        .map(|k| GroundSlot {
            layer: k * 2,
            repeats: k as f32 + 1.0,
            detail_repeats: 0.5,
        })
        .collect();
    let ext = ground_extension(
        Handle::default(),
        Handle::default(),
        Handle::default(),
        &slots,
    );
    assert_eq!(ext.layer_count.x, 19);
    // Slot 16 blends array layer 32, repeated 17 times (Köln paints layer 16 and more).
    assert_eq!(ext.slot_layers[4][0], 32);
    assert_eq!(ext.repeats[4][0], 17.0);
    assert_eq!(ext.detail_repeats[4][2], 0.5);
}

#[test]
fn too_many_slots_are_cut_at_the_limit() {
    let slots = vec![
        GroundSlot {
            layer: 1,
            repeats: 1.0,
            detail_repeats: 1.0,
        };
        MAX_TILE_LAYERS + 5
    ];
    let ext = ground_extension(
        Handle::default(),
        Handle::default(),
        Handle::default(),
        &slots,
    );
    assert_eq!(ext.layer_count.x as usize, MAX_TILE_LAYERS);
}

#[test]
fn the_layer_array_shrinks_to_stay_within_the_budget() {
    assert_eq!(layer_array_size(8, 1024), 1024);
    assert_eq!(layer_array_size(21, 1024), 1024);
    // 55 layers of 1024² with mips are ~307 MB: halved to 512.
    assert_eq!(layer_array_size(55, 1024), 512);
    assert_eq!(layer_array_size(3, 300), 512);
    assert_eq!(layer_array_size(10_000, 1024), 256);
}
