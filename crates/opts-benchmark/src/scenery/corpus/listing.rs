//! Listing the `.sco` files below the installation's `Sceneryobjects` folder.

use std::path::Path;

use anyhow::{Context, Result};

use super::super::normalise_rel;

/// Lists every `.sco` file below the installation's `Sceneryobjects` folder, sorted.
///
/// # Errors
/// The folder is missing.
pub fn list_sco_files(root: &Path) -> Result<Vec<String>> {
    let dir = std::fs::read_dir(root)?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("sceneryobjects"))
        })
        .with_context(|| format!("no Sceneryobjects folder in {}", root.display()))?;
    let mut out = Vec::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p
                .extension()
                .is_some_and(|x| x.to_string_lossy().eq_ignore_ascii_case("sco"))
            {
                if let Ok(rel) = p.strip_prefix(root) {
                    out.push(normalise_rel(&rel.to_string_lossy()));
                }
            }
        }
    }
    out.sort();
    Ok(out)
}
