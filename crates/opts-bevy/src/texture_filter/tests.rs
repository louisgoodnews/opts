//! Tests of the texture-filter setting and the object sampler.

use super::*;

#[test]
fn default_is_16x_trilinear() {
    let d = object_sampler_descriptor(TextureFiltering::default());
    assert_eq!(d.anisotropy_clamp, 16);
    assert_eq!(d.mag_filter, ImageFilterMode::Linear);
    assert_eq!(d.min_filter, ImageFilterMode::Linear);
    assert_eq!(d.mipmap_filter, ImageFilterMode::Linear);
    assert_eq!(d.address_mode_u, ImageAddressMode::Repeat);
    assert_eq!(d.address_mode_v, ImageAddressMode::Repeat);
    assert_eq!(d.address_mode_w, ImageAddressMode::Repeat);
}

#[test]
fn off_is_nearest_without_anisotropy() {
    let d = object_sampler_descriptor(TextureFiltering::Off);
    assert_eq!(d.anisotropy_clamp, 1);
    assert_eq!(d.mag_filter, ImageFilterMode::Nearest);
    assert_eq!(d.mipmap_filter, ImageFilterMode::Nearest);
}

#[test]
fn anisotropy_levels_and_keys_round_trip() {
    for f in TextureFiltering::ALL {
        assert_eq!(TextureFiltering::parse(f.key()), Some(f));
        assert_eq!(TextureFiltering::from_u8(f.to_u8()), f);
        if f.is_filtered() {
            // wgpu requires all filters to be linear when anisotropy is above 1.
            let d = object_sampler_descriptor(f);
            assert_eq!(d.anisotropy_clamp, f.anisotropy());
            assert_eq!(d.min_filter, ImageFilterMode::Linear);
        }
    }
    assert_eq!(TextureFiltering::parse("8X"), Some(TextureFiltering::X8));
    assert_eq!(TextureFiltering::parse("2"), None);
}

#[test]
fn current_setting_is_used_by_the_shared_sampler() {
    // One test mutates the global; other tests only use the pure functions.
    set_texture_filtering(TextureFiltering::X4);
    let ImageSampler::Descriptor(d) = current_object_sampler() else {
        panic!("expected a descriptor");
    };
    assert_eq!(d.anisotropy_clamp, 4);
    set_texture_filtering(TextureFiltering::X16);
}
