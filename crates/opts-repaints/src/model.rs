//! The repaints a vehicle's model offers: its `Colorscheme` `[CTC]` folders and the `.cti` items
//! in them, with their setvars and texture replacements.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use opts_bevy::OmsiInstall;
use opts_core::formats::cti::CtiFile;
use opts_core::formats::vehicles::model::ModelConfig;

use super::folders::dirs_ignoring_case;
use super::VehicleRepaints;

/// Folders a `[CTC]` texture path is resolved against: the vehicle folder, its parent and
/// grandparent, then the installation root.
fn base_folders(bus_dir: &Path, root: &Path) -> Vec<PathBuf> {
    let mut v = vec![bus_dir.to_path_buf()];
    v.extend(bus_dir.ancestors().skip(1).take(2).map(Path::to_path_buf));
    v.push(root.to_path_buf());
    v
}

/// Adds the `.cti` items of `model`'s `Colorscheme` folders to `out`.
pub(super) fn add_model_repaints(
    out: &mut VehicleRepaints,
    root: &Path,
    bus_dir: &Path,
    model: &ModelConfig,
) {
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
