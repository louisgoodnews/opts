//! The corpus entries with their tiers: referenced by a map, attached to a spline, or unreferenced.

use std::path::Path;

use anyhow::Result;

use super::super::{object_key, ObjectEntry};
use super::listing::list_sco_files;
use super::references::map_references;

/// Builds the entries with their tiers.
///
/// # Errors
/// The `Sceneryobjects` folder is missing.
pub fn entries(root: &Path) -> Result<Vec<ObjectEntry>> {
    let files = list_sco_files(root)?;
    let (objects, attached) = map_references(root);
    Ok(files
        .into_iter()
        .map(|sco| {
            let lower = sco.to_lowercase();
            let tier = if objects.contains(&lower) {
                "referenced"
            } else if attached.contains(&lower) {
                "attachment"
            } else {
                "unreferenced"
            };
            ObjectEntry {
                key: object_key(&sco),
                sco,
                tier: tier.into(),
            }
        })
        .collect())
}
