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
    /// The variables the vehicle declares in its `[varnamelist]` files (lower-case name to the
    /// list files); empty when they are not known (a discovery from a model alone).
    pub declared: BTreeMap<String, std::collections::BTreeSet<String>>,
    /// The folder below `Vehicles` the vehicle lives in (the key of the documented setvar
    /// families); empty when unknown.
    pub folder: String,
}

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

/// Adds the `.cti` items of `model`'s `Colorscheme` folders to `out`.
/// Every directory `rel` (backslash or slash separated) names below `base`, comparing names
/// without regard to case: an installation can hold `texture` and `Texture` side by side (they are
/// one folder on Windows, where OMSI runs), and the repaints are in both.
#[must_use]
pub fn dirs_ignoring_case(base: &Path, rel: &str) -> Vec<PathBuf> {
    let mut level = vec![base.to_path_buf()];
    for part in rel
        .split(['\\', '/'])
        .filter(|p| !p.is_empty() && *p != ".")
    {
        let mut next = Vec::new();
        for dir in &level {
            if part == ".." {
                next.extend(dir.parent().map(Path::to_path_buf));
                continue;
            }
            let Ok(entries) = std::fs::read_dir(dir) else {
                continue;
            };
            let mut hits: Vec<PathBuf> = entries
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect();
            hits.sort();
            next.extend(hits);
        }
        level = next;
    }
    level
}

fn add_model_repaints(out: &mut VehicleRepaints, root: &Path, bus_dir: &Path, model: &ModelConfig) {
    let install = OmsiInstall::new(root);
    let mut found: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut textures: BTreeMap<String, Vec<(String, PathBuf)>> = BTreeMap::new();
    for scheme in model
        .ctc_schemes
        .iter()
        .filter(|s| s.name.eq_ignore_ascii_case("Colorscheme"))
    {
        // All the folders the path names (they differ in case only on a case-sensitive file
        // system), the first base that has one.
        let Some(folders) = base_folders(bus_dir, root)
            .iter()
            .map(|b| dirs_ignoring_case(b, &scheme.texture_path))
            .find(|v| !v.is_empty())
        else {
            tracing::error!(folder = %scheme.texture_path, "repaint folder not found");
            continue;
        };
        let mut files: Vec<(PathBuf, PathBuf)> = Vec::new();
        for folder in &folders {
            let Ok(entries) = std::fs::read_dir(folder) else {
                continue;
            };
            files.extend(
                entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("cti")))
                    .map(|p| (folder.clone(), p)),
            );
        }
        files.sort();
        for (folder, file) in files {
            let Ok(bytes) = std::fs::read(&file) else {
                tracing::error!(file = %file.display(), "cannot read .cti");
                continue;
            };
            let cti = CtiFile::parse(&bytes);
            for item in cti.items {
                // The texture lies beside the `.cti` or in the same folder under another case.
                let texture = std::iter::once(&folder)
                    .chain(folders.iter())
                    .find_map(|f| install.find_in(f, &item.texture));
                match texture {
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
}

/// A well-mixed 64-bit hash of `parts` (SplitMix64 steps), stable across runs and platforms:
/// the seed of a per-instance repaint choice.
#[must_use]
pub fn instance_seed(parts: &[u64]) -> u64 {
    let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
    for &p in parts {
        h ^= p;
        h = h.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = h;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        h = z ^ (z >> 31);
    }
    h
}

/// Stable 64-bit hash of a string (FNV-1a), for seeds made from names.
#[must_use]
pub fn name_seed(name: &str) -> u64 {
    name.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
    })
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

impl VehicleRepaints {
    /// The repaint of one placed instance (a parked car, an AI vehicle), chosen deterministically
    /// from `seed` among the default (unpainted when the vehicle names none) and the `.cti`
    /// items. OMSI's own choice rule is unknown; a uniform choice per instance is assumed.
    #[must_use]
    pub fn pick(&self, seed: u64) -> &str {
        let unnamed_default = usize::from(self.default_name.is_empty());
        let count = self.names.len() + unnamed_default;
        if count == 0 {
            return "";
        }
        let i = usize::try_from(seed % count as u64).unwrap_or(0);
        if i < unnamed_default {
            ""
        } else {
            &self.names[i - unnamed_default]
        }
    }

    /// The repaint called `wanted` (case-insensitive), the default's name included; `None` when
    /// the vehicle has no such repaint.
    #[must_use]
    pub fn named(&self, wanted: &str) -> Option<&str> {
        let wanted = wanted.trim();
        if wanted.is_empty() {
            return None;
        }
        if self.default_name.eq_ignore_ascii_case(wanted) {
            return Some(self.default_name.as_str());
        }
        self.names
            .iter()
            .find(|n| n.eq_ignore_ascii_case(wanted))
            .map(String::as_str)
    }

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
    fn folders_that_differ_in_case_are_all_found_and_their_repaints_merge() {
        let root = tmp_root("case");
        let veh = root.join("Vehicles/V");
        std::fs::create_dir_all(veh.join("texture/Ads")).unwrap();
        std::fs::create_dir_all(veh.join("Texture/ads")).unwrap();
        std::fs::write(
            veh.join("texture/Ads/One.cti"),
            "[item]\nOne\nfarbschema_tex1\none.dds\n",
        )
        .unwrap();
        std::fs::write(
            veh.join("Texture/ads/Two.cti"),
            "[item]\nTwo\nfarbschema_tex1\ntwo.dds\n",
        )
        .unwrap();
        std::fs::write(veh.join("Texture/ads/two.dds"), b"x").unwrap();
        let dirs = dirs_ignoring_case(&veh, "Texture\\Ads");
        assert_eq!(dirs.len(), 2, "{dirs:?}");
        assert!(dirs_ignoring_case(&veh, "Texture\\Missing").is_empty());
        let _ = std::fs::remove_dir_all(&root);
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
    fn instance_choice_is_deterministic_and_covers_all_repaints() {
        let r = VehicleRepaints {
            default_name: String::new(),
            names: vec!["Rot".into(), "Gold".into(), "Taxi".into()],
            ..Default::default()
        };
        let a: Vec<&str> = (0..64).map(|i| r.pick(instance_seed(&[7, i]))).collect();
        let b: Vec<&str> = (0..64).map(|i| r.pick(instance_seed(&[7, i]))).collect();
        assert_eq!(a, b);
        for name in ["", "Rot", "Gold", "Taxi"] {
            assert!(a.contains(&name), "{name} chosen at least once");
        }
        assert_ne!(instance_seed(&[1, 2]), instance_seed(&[2, 1]));
        assert_eq!(name_seed("W123"), name_seed("W123"));
        // A named default is one of the choices, not an extra unpainted one.
        let named = VehicleRepaints {
            default_name: "Beige".into(),
            names: vec!["Beige".into()],
            ..Default::default()
        };
        assert_eq!(named.pick(12345), "Beige");
        assert_eq!(VehicleRepaints::default().pick(3), "");
    }

    #[test]
    fn model_repaints_resolve_the_colorscheme_folder_from_the_sco_folder() {
        let root = tmp_root("c");
        let veh = root.join("Vehicles/Car");
        std::fs::create_dir_all(veh.join("Model")).unwrap();
        std::fs::create_dir_all(veh.join("Texture")).unwrap();
        std::fs::write(veh.join("Texture/red.tga"), b"x").unwrap();
        std::fs::write(
            veh.join("Texture/l.cti"),
            "[item]\nRot\nfarbschema\nred.tga\n",
        )
        .unwrap();
        let model = ModelConfig::parse_bytes(
            b"[CTC]\nColorscheme\nTexture\n0\n[CTCTexture]\nfarbschema\nCar_Main.tga\n",
        )
        .unwrap();
        let r = discover_for_model(&root, &veh, &model, "");
        assert_eq!(r.names, ["Rot"]);
        let repl = r.replacements("Rot", &model.ctc_textures);
        assert_eq!(repl.len(), 1);
        assert_eq!(repl[0].0, "car_main.tga");
        assert!(r.replacements("", &model.ctc_textures).is_empty());
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
