//! Every loader path must use the shared object-texture sampler (no private descriptors that
//! fall back to Bevy's nearest-filter defaults).

const LOADER_SOURCES: [(&str, &str); 3] = [
    ("model.rs", include_str!("../src/model.rs")),
    (
        "hot_reload/cache.rs",
        include_str!("../src/hot_reload/cache.rs"),
    ),
    (
        "text_texture/gpu.rs",
        include_str!("../src/text_texture/gpu.rs"),
    ),
];

#[test]
fn loader_paths_use_the_shared_sampler() {
    for (name, src) in LOADER_SOURCES {
        assert!(
            !src.contains("ImageSamplerDescriptor"),
            "{name} builds its own sampler descriptor"
        );
    }
    for (name, src) in &LOADER_SOURCES[..2] {
        assert!(src.contains("prepare_object_texture(&mut image)"), "{name}");
    }
    assert!(LOADER_SOURCES[2]
        .1
        .contains("prepare_text_texture(&mut image)"));
}

#[test]
fn repeat_sampler_follows_the_setting() {
    use opts_bevy::{set_texture_filtering, TextureFiltering};
    set_texture_filtering(TextureFiltering::X8);
    let bevy::image::ImageSampler::Descriptor(d) = opts_bevy::model::repeat_sampler() else {
        panic!("expected a descriptor");
    };
    assert_eq!(d.anisotropy_clamp, 8);
    set_texture_filtering(TextureFiltering::default());
}
