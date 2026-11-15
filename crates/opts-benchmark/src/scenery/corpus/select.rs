//! Selection of entries: tier, glob, explicit list, sampling and limit.

use std::collections::HashSet;

use anyhow::Result;

use super::super::{normalise_rel, ObjectEntry};

/// Selection options.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// `referenced` (placed or attached by a map) or `all`.
    pub tier: String,
    /// Case-insensitive glob on the relative path.
    pub filter: Option<String>,
    /// Explicit list of relative paths (one per line).
    pub list: Option<Vec<String>>,
    /// Every n-th object (by key order) so that about this many remain.
    pub sample: Option<usize>,
    /// At most this many objects.
    pub limit: Option<usize>,
}

/// Applies the selection.
///
/// # Errors
/// An invalid glob.
pub fn select(mut all: Vec<ObjectEntry>, sel: &Selection) -> Result<Vec<ObjectEntry>> {
    if sel.tier == "referenced" {
        all.retain(|e| e.tier != "unreferenced");
    }
    if let Some(pattern) = &sel.filter {
        let glob = globset::GlobBuilder::new(pattern)
            .case_insensitive(true)
            .literal_separator(false)
            .build()?
            .compile_matcher();
        all.retain(|e| glob.is_match(&e.sco));
    }
    if let Some(list) = &sel.list {
        let wanted: HashSet<String> = list
            .iter()
            .map(|s| normalise_rel(s).to_lowercase())
            .collect();
        all.retain(|e| wanted.contains(&e.sco.to_lowercase()));
    }
    if let Some(n) = sel.sample.filter(|n| *n > 0 && *n < all.len()) {
        all.sort_by(|a, b| a.key.cmp(&b.key));
        let step = all.len() as f64 / n as f64;
        all = (0..n)
            .filter_map(|i| all.get((i as f64 * step) as usize).cloned())
            .collect();
    }
    if let Some(n) = sel.limit {
        all.truncate(n);
    }
    Ok(all)
}
