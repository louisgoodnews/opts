//! Helpers of the corpus test: the vehicle's `.hof` and its script textures.

use opts_core::formats::{
    hof::HofFile,
    script_texture::CpuScriptTexture,
    vehicles::{bus::BusManifest, model::ModelConfig},
};

/// The first parsable `.hof` in the vehicle directory.
pub(super) fn find_hof(bus_dir: &std::path::Path) -> Option<std::sync::Arc<HofFile>> {
    let entries = std::fs::read_dir(bus_dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("hof")) {
            if let Ok(hbytes) = std::fs::read(&p) {
                if let Ok(h) = HofFile::parse_bytes(&hbytes) {
                    return Some(std::sync::Arc::new(h));
                }
            }
        }
    }
    None
}

/// Script textures from the model config if available, else eight default ones.
pub(super) fn script_textures(
    install: &opts_bevy::OmsiInstall,
    manifest: &BusManifest,
    bus_dir: &std::path::Path,
) -> Vec<CpuScriptTexture> {
    let mut script_textures = Vec::new();
    if let Some(cfg_rel) = &manifest.model_cfg {
        if let Some(cfg_path) = install.find_in(bus_dir, cfg_rel) {
            if let Ok(cfg_bytes) = std::fs::read(&cfg_path) {
                if let Ok(model) = ModelConfig::parse_bytes(&cfg_bytes) {
                    for st in &model.script_textures {
                        script_textures.push(CpuScriptTexture::new(st.width, st.height));
                    }
                }
            }
        }
    }
    if script_textures.is_empty() {
        for _ in 0..8 {
            script_textures.push(CpuScriptTexture::new(256, 64));
        }
    }
    script_textures
}
