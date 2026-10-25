//! The folder a vehicle's `[mesh]` paths are relative to.
//!
//! The specification (`docs/src/formats/text/vehicles/model_cfg.md`) says the vehicle's `model\`
//! folder. Almost every vehicle keeps its `model.cfg` in that folder,
//! so the config's own folder has served so far; the Studio Polygon Lite buses keep theirs in
//! `Model/Configuration Files/<variant>/` and name the meshes from `Model/` (526 of 527 parts were
//! not found), as do the AI variants of the MB C2 BVG (none found). The config's folder stays the
//! default; the vehicle's `model` folder is taken when it resolves more of the meshes. Used for the
//! player's vehicle, its trailers, AI traffic, parked vehicles and rail units.

use std::path::{Path, PathBuf};

use opts_bevy::OmsiInstall;
use opts_core::formats::vehicles::model::ModelConfig;

/// The base folder for the meshes of `model` (read from `cfg_path`) of the vehicle in `bus_dir`,
/// judged by the meshes of every level of detail.
pub(crate) fn model_mesh_dir(
    install: &OmsiInstall,
    bus_dir: &Path,
    cfg_path: &Path,
    model: &ModelConfig,
) -> PathBuf {
    let meshes: Vec<&str> = model
        .lods
        .iter()
        .flat_map(|l| l.meshes.iter().map(|m| m.path.as_str()))
        .collect();
    mesh_base_dir(install, bus_dir, cfg_path, &meshes)
}

/// The base folder for `meshes` (paths from the `model.cfg` at `cfg_path`) of the vehicle in
/// `bus_dir`.
pub(crate) fn mesh_base_dir(
    install: &OmsiInstall,
    bus_dir: &Path,
    cfg_path: &Path,
    meshes: &[&str],
) -> PathBuf {
    let cfg_dir = cfg_path.parent().map(Path::to_path_buf).unwrap_or_default();
    let Some(model_dir) = install.find_in(bus_dir, "model") else {
        return cfg_dir;
    };
    if model_dir == cfg_dir {
        return cfg_dir;
    }
    let found = |base: &Path| {
        meshes
            .iter()
            .filter(|m| install.find_in(base, m).is_some())
            .count()
    };
    let in_cfg = found(&cfg_dir);
    if in_cfg == meshes.len() {
        return cfg_dir;
    }
    if found(&model_dir) > in_cfg {
        tracing::info!(cfg = %cfg_dir.display(), model = %model_dir.display(), "vehicle meshes are relative to the vehicle's model folder");
        model_dir
    } else {
        cfg_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(name: &str, files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("opts_mesh_dir_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for f in files {
            let p = dir.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"x").unwrap();
        }
        dir
    }

    #[test]
    fn meshes_next_to_the_config_keep_the_config_folder() {
        let dir = tree("next", &["Bus/model/model.cfg", "Bus/model/body.o3d"]);
        let install = OmsiInstall::new(&dir);
        let bus = dir.join("Bus");
        let base = mesh_base_dir(&install, &bus, &bus.join("model/model.cfg"), &["body.o3d"]);
        assert_eq!(base, bus.join("model"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_config_in_a_subfolder_takes_its_meshes_from_the_model_folder() {
        let dir = tree(
            "sub",
            &[
                "Bus/Model/Configuration Files/Regional/a.cfg",
                "Bus/Model/Exterior/Body/body.o3d",
                "Bus/Model/Exterior/Body/door.o3d",
            ],
        );
        let install = OmsiInstall::new(&dir);
        let bus = dir.join("Bus");
        let cfg = bus.join("Model/Configuration Files/Regional/a.cfg");
        let base = mesh_base_dir(
            &install,
            &bus,
            &cfg,
            &["Exterior\\Body\\body.o3d", "Exterior\\Body\\door.o3d"],
        );
        assert_eq!(base, bus.join("Model"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_config_folder_that_finds_as_many_stays() {
        let dir = tree(
            "tie",
            &[
                "Bus/model/ai/a.cfg",
                "Bus/model/ai/x.o3d",
                "Bus/model/y.o3d",
            ],
        );
        let install = OmsiInstall::new(&dir);
        let bus = dir.join("Bus");
        let base = mesh_base_dir(
            &install,
            &bus,
            &bus.join("model/ai/a.cfg"),
            &["x.o3d", "y.o3d"],
        );
        assert_eq!(
            base,
            bus.join("model/ai"),
            "one each: the config's folder stays"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
