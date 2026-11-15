//! Repaint discovery for the vehicle selection tab.
//!
//! The chain is the one the game follows (`docs/src/formats/text/repaints.md`): the vehicle's
//! `.bus` names its `model.cfg`; the `[CTC]` blocks named `Colorscheme` there name a texture folder;
//! every `.cti` file of that folder lists repaints as `[item]` names. The `.bus` file's third
//! `[friendlyname]` line is the default repaint (shown while no texture change is active).

mod discover;
mod folders;
mod model;
mod seed;

use std::collections::BTreeMap;
use std::path::PathBuf;

pub use discover::{discover, discover_at, discover_for_model, family_folder, vehicle_repaints};
pub use folders::{dirs_ignoring_case, texture_dirs};
pub use seed::{instance_seed, name_seed};

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
mod tests;
