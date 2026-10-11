//! Repaint discovery for the vehicle selection tab.
//!
//! The chain is the one the game follows (`docs/src/formats/text/repaints.md`): the vehicle's
//! `.bus` names its `model.cfg`; the `[CTC]` blocks named `Colorscheme` there name a texture folder;
//! every `.cti` file of that folder lists repaints as `[item]` names. The `.bus` file's third
//! `[friendlyname]` line is the default repaint (shown while no texture change is active).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use opts_bevy::OmsiInstall;
use opts_core::formats::cti::CtiFile;
use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;

use crate::new_game_screen::types::VehicleItem;

/// Repaints a vehicle offers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VehicleRepaints {
    /// Default repaint from the `.bus` file (first in [`Self::names`] when set).
    pub default_name: String,
    /// Selectable repaint names: the default first, then the `.cti` items alphabetically.
    pub names: Vec<String>,
    /// `[setvar]` lines per repaint name (variable, value).
    pub setvars: BTreeMap<String, Vec<(String, String)>>,
    /// Texture files per repaint name and `[CTCTexture]` slot (resolved paths).
    pub textures: BTreeMap<String, Vec<(String, PathBuf)>>,
}

/// Folders a `[CTC]` texture path is resolved against: the vehicle folder, its parent and
/// grandparent, then the installation root.
fn base_folders(bus_dir: &Path, root: &Path) -> Vec<PathBuf> {
    let mut v = vec![bus_dir.to_path_buf()];
    v.extend(bus_dir.ancestors().skip(1).take(2).map(Path::to_path_buf));
    v.push(root.to_path_buf());
    v
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
    let model = (|| -> Result<ModelConfig, String> {
        let manifest =
            BusManifest::parse_bytes(&std::fs::read(bus_path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let rel = manifest.model_cfg.ok_or("no [model] entry")?;
        let cfg = install
            .find_in(&bus_dir, &rel)
            .ok_or_else(|| format!("model config not found: {rel}"))?;
        ModelConfig::parse_bytes(&std::fs::read(cfg).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    })();
    let model = match model {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(bus = %bus_path.display(), "cannot read model config for repaints: {e}");
            return out;
        }
    };
    let mut found: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut textures: BTreeMap<String, Vec<(String, PathBuf)>> = BTreeMap::new();
    for scheme in model
        .ctc_schemes
        .iter()
        .filter(|s| s.name.eq_ignore_ascii_case("Colorscheme"))
    {
        let Some(folder) = base_folders(&bus_dir, root)
            .iter()
            .find_map(|b| install.find_in(b, &scheme.texture_path))
            .filter(|p| p.is_dir())
        else {
            tracing::error!(folder = %scheme.texture_path, "repaint folder not found");
            continue;
        };
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("cti")))
            .collect();
        files.sort();
        for file in files {
            let Ok(bytes) = std::fs::read(&file) else {
                tracing::error!(file = %file.display(), "cannot read .cti");
                continue;
            };
            let cti = CtiFile::parse(&bytes);
            for item in cti.items {
                match install.find_in(&folder, &item.texture) {
                    Some(path) => textures
                        .entry(item.name.clone())
                        .or_default()
                        .push((item.slot, path)),
                    None => tracing::debug!(
                        repaint = %item.name,
                        texture = %item.texture,
                        folder = %folder.display(),
                        "repaint texture not found"
                    ),
                }
                found.entry(item.name).or_default();
            }
            for sv in cti.setvars {
                found
                    .entry(sv.repaint)
                    .or_default()
                    .push((sv.variable, sv.value));
            }
        }
    }
    out.textures = textures;
    for (name, setvars) in found {
        if !name.is_empty() && !out.names.contains(&name) {
            out.names.push(name.clone());
        }
        out.setvars.entry(name).or_default().extend(setvars);
    }
    out
}

/// Repaints of a menu vehicle (empty list when the installation is unknown).
#[must_use]
pub fn vehicle_repaints(vehicle: &VehicleItem) -> VehicleRepaints {
    match crate::omsi_root::known_omsi_root() {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("opts_repaints_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn default_first_then_cti_items_of_the_colorscheme_folder() {
        let root = tmp_root("a");
        let veh = root.join("Vehicles/V");
        std::fs::create_dir_all(veh.join("Model")).unwrap();
        std::fs::create_dir_all(veh.join("Texture/Ads")).unwrap();
        std::fs::write(
            veh.join("v.bus"),
            "[friendlyname]\nMAN\nX\nBeige\n[model]\nModel\\m.cfg\n",
        )
        .unwrap();
        std::fs::write(
            veh.join("Model/m.cfg"),
            "[CTC]\nColorscheme\nTexture\\Ads\n0\n[CTCTexture]\ntex1\na.tga\n[CTC]\nOther\nTexture\n0\n",
        )
        .unwrap();
        std::fs::write(
            veh.join("Texture/Ads/b.CTI"),
            "[item]\n Zebra\ntex1\nz.tga\n[setvar]\nlights\n1\n[item]\nBeige\ntex1\nb.tga\n[item]\nAlpha\ntex1\nal.tga\n",
        )
        .unwrap();
        let r = discover(&root, "Vehicles/V/v.bus", "Beige");
        assert_eq!(r.names, ["Beige", "Alpha", "Zebra"]);
        assert_eq!(
            r.setvars["Zebra"],
            [("lights".to_string(), "1".to_string())]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_bus_lists_only_the_default() {
        let root = tmp_root("b");
        let r = discover(&root, "Vehicles/none.bus", "Beige");
        assert_eq!(r.names, ["Beige"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Needs an installation: `OMSI_PATH=<root> cargo test -p opts-game repaints -- --ignored`.
    #[test]
    #[ignore]
    fn real_installation_lists_gn92_repaints() {
        let root = PathBuf::from(std::env::var("OMSI_PATH").expect("OMSI_PATH"));
        let r = discover(&root, "Vehicles/MAN_NL_NG/MAN_GN92_main.bus", "Beige");
        eprintln!("{} repaints: {:?}", r.names.len(), r.names);
        assert_eq!(r.names[0], "Beige");
        assert!(r.names.len() > 5);
    }
}

impl VehicleRepaints {
    /// Texture replacements of repaint `name` for a model whose `[CTCTexture]` slots declare the
    /// default texture file of each slot: `(default file name, replacement path)`. Empty for the
    /// default repaint and for unknown names.
    #[must_use]
    pub fn replacements(
        &self,
        name: &str,
        slots: &[opts_core::formats::vehicles::model::CtcTexture],
    ) -> Vec<(String, PathBuf)> {
        if name == self.default_name {
            return Vec::new();
        }
        let Some(items) = self.textures.get(name) else {
            return Vec::new();
        };
        slots
            .iter()
            .filter_map(|slot| {
                let (_, path) = items
                    .iter()
                    .find(|(s, _)| s.eq_ignore_ascii_case(&slot.name))?;
                let default = slot
                    .texture_path
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or("")
                    .to_lowercase();
                Some((default, path.clone()))
            })
            .collect()
    }
}
