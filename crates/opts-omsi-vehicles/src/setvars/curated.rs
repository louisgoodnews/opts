//! The vendor's setvar documents (`data/setvar_families.json`): the families, their setvars and
//! the plain value ranges they document.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::Deserialize;

/// A documented range or list of whole numbers: `0|1`, `0-3`, `0,1,10`; anything else (`list`,
/// `LINE 0|1 ; CITY 0-2`) is not a plain range and gives `None`.
#[must_use]
pub fn parse_range(text: &str) -> Option<BTreeSet<i64>> {
    let t = text.trim();
    if let Some((a, b)) = t.split_once('-') {
        let (a, b): (i64, i64) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
        return (a <= b && b - a <= 1000).then(|| (a..=b).collect());
    }
    let set: Option<BTreeSet<i64>> = t.split(['|', ',']).map(|p| p.trim().parse().ok()).collect();
    set.filter(|s| !s.is_empty())
}

/// One documented setvar of a vendor document.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CuratedSetvar {
    /// The script variable.
    pub name: String,
    /// The documented values as text (`0|1`, `0-3`, `list`, ...).
    #[serde(default)]
    pub values: String,
    /// A short meaning in our own words.
    #[serde(default)]
    pub meaning: String,
    /// Vendor group.
    #[serde(default)]
    pub group: String,
    /// `true`: the vendor says the vehicle must be built again after a change; `false`: a live
    /// test showed the running vehicle reacts; absent: not known (Q-SV02).
    #[serde(default)]
    pub respawn: Option<bool>,
}

/// The setvars one vendor documents for a family of vehicles.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CuratedFamily {
    /// Stable id (`citybus-o530-kajosoft`).
    pub id: String,
    /// The family's name.
    pub title: String,
    /// Folders below `Vehicles` that belong to the family.
    #[serde(default)]
    pub folders: Vec<String>,
    /// The documented setvars.
    pub setvars: Vec<CuratedSetvar>,
}

#[derive(Deserialize)]
struct CuratedFile {
    families: Vec<CuratedFamily>,
}

/// The documented families (parsed once from the data file shipped with the program).
#[must_use]
pub fn curated_families() -> &'static [CuratedFamily] {
    static FAMILIES: OnceLock<Vec<CuratedFamily>> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        serde_json::from_str::<CuratedFile>(include_str!("../../data/setvar_families.json"))
            .map(|f| f.families)
            .unwrap_or_default()
    })
}

/// The documented family of a vehicle folder below `Vehicles` (case-insensitive).
#[must_use]
pub fn family_of_folder(folder: &str) -> Option<&'static CuratedFamily> {
    curated_families().iter().find(|f| {
        f.folders
            .iter()
            .any(|x| x.trim().eq_ignore_ascii_case(folder.trim()))
    })
}
