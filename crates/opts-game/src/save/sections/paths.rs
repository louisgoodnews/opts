//! Checks on the paths a save file names. A save is untrusted input: its map folder must be one
//! plain folder name, and an absolute `.bus` path must lie inside the save folder or the OMSI
//! folder's `maps` and `Vehicles` folders.

use std::path::{Component, Path};

use crate::save::SaveStore;

/// The map folder a save names: exactly one normal folder name, with no separator, root, drive
/// prefix, `.` or `..`.
///
/// # Errors
/// A message naming the rejected value; the caller returns it as the section's error.
pub(super) fn map_folder_name(raw: &str) -> Result<&str, String> {
    let mut parts = Path::new(raw).components();
    let single = matches!(
        (parts.next(), parts.next()),
        (Some(Component::Normal(_)), None)
    );
    if single {
        return Ok(raw);
    }
    tracing::warn!(map_folder = raw, "save refused: the map folder is not one folder name");
    Err(format!("the map folder `{raw}` is not one folder name"))
}

/// Whether a `.bus` path names a place a save may point at. A relative path is always allowed
/// (it is resolved below the installation). An absolute one must have no `..` and lie inside the
/// save folder, or inside `maps` or `Vehicles` of the installation at `root`.
#[must_use]
pub(super) fn bus_path_allowed(saved: &Path, root: Option<&Path>) -> bool {
    if !saved.is_absolute() {
        return true;
    }
    let escapes = saved.components().any(|c| c == Component::ParentDir);
    let omsi = root
        .into_iter()
        .flat_map(|r| [r.join("maps"), r.join("Vehicles")]);
    let inside = SaveStore::default_dir()
        .into_iter()
        .chain(omsi)
        .any(|dir| saved.starts_with(dir));
    let allowed = !escapes && inside;
    if !allowed {
        tracing::warn!(
            saved = %saved.display(),
            "save refused: a bus file lies outside the save and OMSI folders"
        );
    }
    allowed
}
