//! AI traffic lists of a map folder (`docs/src/formats/map/lists.md`): `ailists.cfg`,
//! `unsched_vehgroups.txt` and `unsched_trafficdens.txt`.
//!
//! These files say *which* vehicles may appear as AI traffic and *how dense* that traffic is over
//! the day. They do not say where vehicles drive: that comes from the `[path]` lanes of splines and
//! junction objects (`opts-traffic`). The depot groups' vehicle files are kept in
//! [`AiLists::depot_vehicles`]; the legacy `[ailist]`/`[aigroup]` syntax is read into `groups`.

use crate::formats::text::{decode_bytes, scan_blocks};

use opts_omsi_ailist::legacy;

mod depot;
pub use depot::{DepotGroup, FleetEntry};
/// The AI group types live in `opts-omsi-ailist` (with the legacy parser); re-exported here.
pub use opts_omsi_ailist::{AiEntry, AiGroup};

/// Parsed `ailists.cfg` (current `_2` syntax only).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AiLists {
    /// Groups in file order.
    pub groups: Vec<AiGroup>,
    /// Vehicle files of the depot groups (`[aigroup_depot_typgroup_2]`: the operator's own fleet,
    /// first line of each block), duplicates removed, in file order.
    pub depot_vehicles: Vec<String>,
    /// The depot groups with their vehicles: the `[aigroup_depot_typgroup_2]` blocks after an
    /// `[aigroup_depot]` block belong to it.
    pub depot_groups: Vec<DepotGroup>,
}

impl AiLists {
    fn handle_aigroup(parameters: &[String], out: &mut Self) {
        let mut lines = parameters.iter();
        let Some(name) = lines.next() else { return };
        let entries = lines
            .filter_map(|l| {
                let (file, weight) = match l.rsplit_once('\t') {
                    Some((f, w)) => (f.trim(), w.trim().parse::<f32>().ok()),
                    None => (l.trim(), None),
                };
                let lower = file.to_ascii_lowercase();
                (lower.ends_with(".bus") || lower.ends_with(".ovh") || lower.ends_with(".zug"))
                    .then(|| AiEntry {
                        vehicle: file.to_owned(),
                        weight,
                    })
            })
            .collect();
        out.groups.push(AiGroup {
            name: name.trim().to_owned(),
            entries,
        });
    }

    /// Parses `ailists.cfg`: the `_2` groups and the legacy `[ailist]`/`[aigroup]` pair. Lines that
    /// do not look like `<file>[<TAB><weight>]` are skipped.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        let mut out = Self::default();
        let mut legacy_table = Vec::new();
        for block in scan_blocks(&decode_bytes(bytes)) {
            match block.keyword.as_str() {
                "aigroup_depot" => Self::handle_depot(&block.parameters, &mut out),
                "aigroup_depot_typgroup_2" => {
                    Self::handle_depot_typgroup(&block.parameters, &mut out);
                }
                "aigroup_2" => Self::handle_aigroup(&block.parameters, &mut out),
                "ailist" => legacy_table = legacy::vehicle_table(&block.parameters),
                "aigroup" => out
                    .groups
                    .extend(legacy::group(&block.parameters, &legacy_table)),
                _ => {}
            }
        }
        out
    }

    /// The depot group called `name` (case-insensitive).
    #[must_use]
    pub fn depot_group(&self, name: &str) -> Option<&DepotGroup> {
        self.depot_groups
            .iter()
            .find(|g| g.name.eq_ignore_ascii_case(name))
    }

    /// The group called `name` (case-insensitive).
    #[must_use]
    pub fn group(&self, name: &str) -> Option<&AiGroup> {
        self.groups
            .iter()
            .find(|g| g.name.eq_ignore_ascii_case(name))
    }
}

/// One entry of `unsched_vehgroups.txt`.
#[derive(Debug, Clone, PartialEq)]
pub struct UnschedGroup {
    /// AI group name.
    pub name: String,
    /// Default density class: `0` no traffic on a path unless freed, `1` normal, higher values
    /// select further density classes.
    pub default_density: u32,
}

/// Parses `unsched_vehgroups.txt` (`[group]`: name, default density).
#[must_use]
pub fn parse_unsched_vehgroups(bytes: &[u8]) -> Vec<UnschedGroup> {
    scan_blocks(&decode_bytes(bytes))
        .into_iter()
        .filter(|b| b.keyword == "group")
        .filter_map(|b| {
            let name = b.parameters.first()?.trim().to_owned();
            let default_density = b.parameters.get(1)?.trim().parse().ok()?;
            Some(UnschedGroup {
                name,
                default_density,
            })
        })
        .collect()
}

pub use density::*;
mod density;


#[cfg(test)]
mod tests;
