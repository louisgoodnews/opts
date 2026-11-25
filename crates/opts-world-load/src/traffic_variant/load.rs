//! Loading one vehicle file of the AI list: its manifest, constant files, profile and model.

use std::path::{Path, PathBuf};

use super::*;
use opts_core::formats::vehicles::BusManifest;

/// Reads one vehicle file: manifest, constant files, profile and model.
pub(super) fn load_unit(
    job: &WorldLoadJob,
    path: &Path,
) -> Result<(Unit, BusManifest, PathBuf), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let manifest = BusManifest::parse_bytes(&bytes).map_err(|e| e.to_string())?;
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let cfg_rel = manifest.model_cfg.clone().ok_or("no [model] entry")?;
    let cfg = job
        .install
        .find_in(&dir, &cfg_rel)
        .ok_or_else(|| format!("model config not found: {cfg_rel}"))?;
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let model_dir = super::super::mesh_dir::model_mesh_dir(&job.install, &dir, &cfg, &model);
    let consts: Vec<ConstFile> = manifest
        .constfiles
        .iter()
        .filter_map(|c| job.install.find_in(&dir, c))
        .filter_map(|p| std::fs::read(p).ok())
        .map(|b| ConstFile::parse(&b))
        .collect();
    let profile = AiProfile::from_manifest(&manifest, &consts);
    let response = measure_script(job, &manifest, &dir);
    let front_axle_y = manifest
        .axles
        .iter()
        .map(|a| a.longitudinal_m)
        .reduce(f32::max);
    // Every `texture` folder, whatever the case (`Texture` and `texture` can sit side by side).
    let tex_dirs = opts_repaints::texture_dirs(&dir, &model_dir);
    let default_livery = manifest
        .friendly_name
        .as_ref()
        .map(|f| f.livery.clone())
        .unwrap_or_default();
    let repaints = opts_repaints::discover_for_model(
        &job.config.root,
        &dir,
        &model,
        &default_livery,
    );
    let unit = Unit {
        bus_path: path.to_path_buf(),
        name: path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        model,
        model_dir,
        tex_dirs,
        profile,
        response,
        size: None,
        hitch: Vec2::ZERO,
        joint: Vec2::ZERO,
        rear_axle_y: 0.0,
        front_axle_y,
        max_alpha_deg: 52.5,
        repaints,
        reversed: false,
        sound: crate::vehicle_sound::ai_sound_set(job, &manifest, &dir, path),
        front_door_y: front_door_y(job, &manifest, &dir),
    };
    Ok((unit, manifest, dir))
}
