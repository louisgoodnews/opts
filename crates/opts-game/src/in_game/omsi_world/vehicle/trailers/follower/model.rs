//! The unit's model: its config file, the job's textures and the rear axle position.

use super::*;

/// Reads the unit's model config: the file and its parsed content.
pub(super) fn load_model(
    job: &WorldLoadJob,
    manifest: &BusManifest,
    dir: &Path,
) -> Result<(PathBuf, ModelConfig), String> {
    let cfg_rel = manifest.model_cfg.clone().ok_or("no [model] entry")?;
    let cfg = job
        .install
        .find_in(dir, &cfg_rel)
        .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok((cfg, model))
}

/// Hands the model's text and script textures to the job, with fresh script canvases.
pub(super) fn set_job_textures(job: &mut WorldLoadJob, model: &ModelConfig) {
    job.vehicle_text_defs = model.text_textures.clone();
    job.vehicle_script_defs = model.script_textures.clone();
    let canvases: Vec<opts_core::formats::script_texture::CpuScriptTexture> = model
        .script_textures
        .iter()
        .map(|st| opts_core::formats::script_texture::CpuScriptTexture::new(st.width, st.height))
        .collect();
    job.vehicle_script_textures = std::sync::Arc::new(std::sync::RwLock::new(canvases));
}

/// Y of the unit's rear axle in its own frame; `front_y` is the link to the unit in front.
pub(super) fn rear_axle_y(manifest: &BusManifest, reversed: bool, front_y: f32) -> f32 {
    let rear_axle_y = if reversed {
        // Mirrored: the model's frontmost axle becomes the rearmost.
        -manifest
            .axles
            .iter()
            .map(|a| a.longitudinal_m)
            .fold(f32::NEG_INFINITY, f32::max)
    } else {
        manifest
            .axles
            .iter()
            .map(|a| a.longitudinal_m)
            .fold(f32::INFINITY, f32::min)
    };
    if rear_axle_y.is_finite() {
        rear_axle_y
    } else {
        front_y - 6.0
    }
}
