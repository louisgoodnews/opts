//! Discovery of a vehicle's repaints from its `.bus` file and from its model config.

use std::collections::BTreeMap;
use std::path::Path;

use opts_bevy::OmsiInstall;
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;

use super::model::add_model_repaints;
use super::VehicleRepaints;
use opts_new_game_types::types::VehicleItem;

/// The first folder below `Vehicles` of a vehicle file or folder (the vendor's family folder).
#[must_use]
pub fn family_folder(path: &Path) -> String {
    let mut parts = path.components().map(|c| c.as_os_str().to_string_lossy());
    parts
        .by_ref()
        .find(|c| c.eq_ignore_ascii_case("Vehicles"))
        .and_then(|_| parts.next())
        .map(|c| c.into_owned())
        .unwrap_or_default()
}

/// The names the `[varnamelist]` files of a vehicle declare, each with the file that declares it.
fn declared_names(
    install: &OmsiInstall,
    bus_dir: &Path,
    lists: &[String],
) -> BTreeMap<String, std::collections::BTreeSet<String>> {
    let mut out: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    for rel in lists {
        let Some(path) = install
            .find_in(bus_dir, rel)
            .or_else(|| install.find_file(rel))
        else {
            continue;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        for name in opts_core::formats::varlist::VarList::parse(&bytes).names {
            out.entry(name.to_lowercase())
                .or_default()
                .insert(file.clone());
        }
    }
    out
}

/// Discovers the repaints of a vehicle under `root`; `bus_file` is the `.bus` path relative to it.
#[must_use]
pub fn discover(root: &Path, bus_file: &str, default_name: &str) -> VehicleRepaints {
    let install = OmsiInstall::new(root);
    match install.find_file(bus_file) {
        Some(bus_path) => discover_at(root, &bus_path, default_name),
        None => {
            tracing::error!(bus = bus_file, "vehicle .bus not found; no repaints listed");
            VehicleRepaints {
                default_name: default_name.to_string(),
                names: if default_name.is_empty() {
                    Vec::new()
                } else {
                    vec![default_name.to_string()]
                },
                ..Default::default()
            }
        }
    }
}

/// Discovers the repaints of the vehicle whose `.bus` file is `bus_path`.
#[must_use]
pub fn discover_at(root: &Path, bus_path: &Path, default_name: &str) -> VehicleRepaints {
    let mut out = VehicleRepaints {
        default_name: default_name.to_string(),
        ..Default::default()
    };
    if !default_name.is_empty() {
        out.names.push(default_name.to_string());
    }
    let install = OmsiInstall::new(root);
    let bus_dir = bus_path.parent().map(Path::to_path_buf).unwrap_or_default();
    out.folder = family_folder(bus_path);
    let model = (|| -> Result<(ModelConfig, BTreeMap<String, std::collections::BTreeSet<String>>), String> {
        let manifest =
            BusManifest::parse_bytes(&std::fs::read(bus_path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let declared = declared_names(&install, &bus_dir, &manifest.varnamelists);
        let rel = manifest.model_cfg.ok_or("no [model] entry")?;
        let cfg = install
            .find_in(&bus_dir, &rel)
            .ok_or_else(|| format!("model config not found: {rel}"))?;
        let model = ModelConfig::parse_bytes(&std::fs::read(cfg).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        Ok((model, declared))
    })();
    let model = match model {
        Ok((m, declared)) => {
            out.declared = declared;
            m
        }
        Err(e) => {
            tracing::error!(bus = %bus_path.display(), "cannot read model config for repaints: {e}");
            return out;
        }
    };
    add_model_repaints(&mut out, root, &bus_dir, &model);
    out
}

/// Discovers the repaints of an already parsed vehicle model: `vehicle_dir` is the folder of the
/// `.bus` or of the parked/static `.sco` that names it (the `[CTC]` folder is resolved against it,
/// as for the player's vehicle). `default_name` may be empty (no named default repaint).
#[must_use]
pub fn discover_for_model(
    root: &Path,
    vehicle_dir: &Path,
    model: &ModelConfig,
    default_name: &str,
) -> VehicleRepaints {
    let mut out = VehicleRepaints {
        default_name: default_name.to_string(),
        folder: family_folder(vehicle_dir),
        ..Default::default()
    };
    if !default_name.is_empty() {
        out.names.push(default_name.to_string());
    }
    add_model_repaints(&mut out, root, vehicle_dir, model);
    out
}

/// Repaints of a menu vehicle (empty list when the installation is unknown).
#[must_use]
pub fn vehicle_repaints(vehicle: &VehicleItem) -> VehicleRepaints {
    match opts_steam::omsi_root::known_omsi_root() {
        Some(root) if !vehicle.bus_file.is_empty() => {
            discover(&root, &vehicle.bus_file, &vehicle.default_livery)
        }
        _ => VehicleRepaints {
            default_name: vehicle.default_livery.clone(),
            names: if vehicle.default_livery.is_empty() {
                Vec::new()
            } else {
                vec![vehicle.default_livery.clone()]
            },
            ..Default::default()
        },
    }
}
